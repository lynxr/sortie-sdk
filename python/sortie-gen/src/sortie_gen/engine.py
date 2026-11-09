"""The engine client: ``sortie.v1`` over gRPC (TCP on loopback; grpcio has no named pipes).

Every call goes through :meth:`EngineClient._call`, which turns gRPC failures into
:class:`~sortie_gen.errors.EngineError` subclasses with a message that says what to do: start
``sortied``, pass a token, or wait for the milestone that implements the feature.
"""

from __future__ import annotations

import os
import re
from collections.abc import Callable, Iterable, Iterator
from dataclasses import dataclass
from typing import Any, TypeVar

import grpc

from .errors import EngineError, EngineNotReady, EngineUnavailable, NotYet
from ._proto.sortie.v1 import engine_pb2, engine_pb2_grpc
from ._proto.sortie.v1 import events_pb2, events_pb2_grpc
from ._proto.sortie.v1 import generator_pb2, generator_pb2_grpc
from ._proto.sortie.v1 import missions_pb2, missions_pb2_grpc

DEFAULT_ENGINE = "127.0.0.1:50777"
ENGINE_ENV = "SORTIE_ENGINE"
TOKEN_ENV = "SORTIE_TOKEN"
CHUNK_BYTES = 1024 * 1024
#: Most type names one ``Generator.ValidateTypes`` request may carry (the engine's limit).
MAX_VALIDATE_TYPES = 512
_MILESTONE_RE = re.compile(r"lands in (M\d+)")

T = TypeVar("T")


def resolve_endpoint(endpoint: str | None = None) -> tuple[str, bool]:
    """``(host:port, tls)`` from an argument, ``SORTIE_ENGINE`` or the default.

    Accepts ``127.0.0.1:50777``, ``http://127.0.0.1:50777`` and ``https://host:port``.
    """
    raw = (endpoint or os.environ.get(ENGINE_ENV) or DEFAULT_ENGINE).strip()
    tls = raw.startswith("https://")
    for prefix in ("http://", "https://", "grpc://"):
        if raw.startswith(prefix):
            raw = raw[len(prefix) :]
    raw = raw.rstrip("/")
    if not raw:
        raise EngineError("the engine address is empty (expected host:port, e.g. 127.0.0.1:50777)")
    return raw, tls


@dataclass
class RegisteredBuild:
    build_id: str
    path: str
    digest: str


class EngineClient:
    """A small synchronous client for the parts of ``sortie.v1`` the generator uses."""

    def __init__(self, endpoint: str | None = None, token: str | None = None, timeout: float = 15.0) -> None:
        self.address, tls = resolve_endpoint(endpoint)
        self.token = token if token is not None else os.environ.get(TOKEN_ENV)
        self.timeout = timeout
        options = [
            ("grpc.max_send_message_length", 64 * 1024 * 1024),
            ("grpc.max_receive_message_length", 64 * 1024 * 1024),
        ]
        if tls:
            self.channel = grpc.secure_channel(self.address, grpc.ssl_channel_credentials(), options=options)
        else:
            self.channel = grpc.insecure_channel(self.address, options=options)
        self.engine = engine_pb2_grpc.EngineStub(self.channel)
        self.missions = missions_pb2_grpc.MissionsStub(self.channel)
        self.generator = generator_pb2_grpc.GeneratorStub(self.channel)
        self.events_stub = events_pb2_grpc.EventsStub(self.channel)

    def close(self) -> None:
        self.channel.close()

    def __enter__(self) -> EngineClient:
        return self

    def __exit__(self, *exc: object) -> None:
        self.close()

    @property
    def metadata(self) -> list[tuple[str, str]]:
        return [("authorization", f"Bearer {self.token}")] if self.token else []

    # ----- error mapping -------------------------------------------------------------------

    def explain(self, method: str, err: grpc.RpcError, timeout: float | None = None) -> EngineError:
        code = err.code() if hasattr(err, "code") else None
        details = (err.details() if hasattr(err, "details") else None) or ""
        if code == grpc.StatusCode.UNAVAILABLE:
            return EngineUnavailable(
                f"cannot reach the Sortie engine at {self.address} ({details or 'connection refused'}). "
                "Is sortied running? Start it with `sortied --dev` for local work, "
                f"or point --engine / {ENGINE_ENV} at it."
            )
        if code == grpc.StatusCode.UNIMPLEMENTED:
            m = _MILESTONE_RE.search(details)
            return NotYet(method, m.group(1) if m else None, details or f"{method} is not implemented by this engine")
        if code == grpc.StatusCode.UNAUTHENTICATED:
            return EngineError(
                f"the engine refused {method}: {details}. Pass --token or set {TOKEN_ENV} to a `generator` token "
                "from <Saved Games>/Sortie/tokens.toml (or run `sortied --dev` on this machine)."
            )
        if code == grpc.StatusCode.PERMISSION_DENIED:
            return EngineError(f"the engine refused {method}: {details} (the generator needs a `generator` or `admin` token)")
        if code == grpc.StatusCode.DEADLINE_EXCEEDED:
            waited = timeout if timeout is not None else self.timeout
            return EngineError(f"{method}: the engine at {self.address} did not answer within {waited:g} s")
        if code == grpc.StatusCode.FAILED_PRECONDITION:
            return EngineNotReady(method, details)
        name = code.name if code is not None else "ERROR"
        return EngineError(f"{method} failed: {name}: {details}")

    def _call(self, method: str, fn: Callable[..., T], request: Any, *, timeout: float | None = None) -> T:
        timeout = timeout if timeout is not None else self.timeout
        try:
            return fn(request, metadata=self.metadata, timeout=timeout)
        except grpc.RpcError as e:
            raise self.explain(method, e, timeout) from None

    # ----- calls ---------------------------------------------------------------------------

    def version(self) -> engine_pb2.VersionResponse:
        return self._call("Engine.Version", self.engine.Version, engine_pb2.VersionRequest())

    def health(self) -> engine_pb2.HealthResponse:
        return self._call("Engine.Health", self.engine.Health, engine_pb2.HealthRequest())

    def begin_build(self, scenario: str, schema: int, schema_sha256: str, generator_version: str) -> str:
        r = self._call(
            "Generator.BeginBuild",
            self.generator.BeginBuild,
            generator_pb2.BeginBuildRequest(
                scenario=scenario, schema=schema, schema_sha256=schema_sha256, generator_version=generator_version
            ),
        )
        return r.build_id

    def validate_types(self, type_names: Iterable[str]) -> generator_pb2.ValidateTypesResponse:
        """Asks the mission running in DCS whether it knows each type (sorted, each once).

        The engine answers one ``TypeCheck`` per name; more than :data:`MAX_VALIDATE_TYPES`
        names go in several requests. Raises :class:`EngineNotReady` when the check cannot run
        (no mission, DCS paused) and :class:`NotYet` from an engine older than M2.
        """
        names = sorted(set(type_names))
        out = generator_pb2.ValidateTypesResponse()
        for i in range(0, len(names), MAX_VALIDATE_TYPES):
            r = self._call(
                "Generator.ValidateTypes",
                self.generator.ValidateTypes,
                generator_pb2.ValidateTypesRequest(type_names=names[i : i + MAX_VALIDATE_TYPES]),
                # Two rounds of lookups in DCS (unit types, then static types), 10 s each at most.
                timeout=max(self.timeout, 30.0),
            )
            out.results.extend(r.results)
        return out

    def register(self, build_id: str, manifest_pb: bytes, miz: bytes, file_name: str) -> RegisteredBuild:
        """Streams ``RegisterHeader`` and the ``.miz`` in chunks of at most 1 MiB."""
        import hashlib

        header = missions_pb2.RegisterHeader(
            build_id=build_id,
            manifest_pb=manifest_pb,
            miz_sha256=hashlib.sha256(miz).hexdigest(),
            miz_size=len(miz),
            file_name=file_name,
        )

        def parts() -> Iterator[missions_pb2.RegisterRequest]:
            yield missions_pb2.RegisterRequest(header=header)
            for i in range(0, len(miz), CHUNK_BYTES):
                yield missions_pb2.RegisterRequest(miz_chunk=miz[i : i + CHUNK_BYTES])

        r = self._call("Missions.Register", self.missions.Register, parts(), timeout=max(self.timeout, 120.0))
        return RegisteredBuild(build_id=r.build_id, path=r.path, digest=r.digest)

    def launch(self, build_id: str) -> missions_pb2.LaunchResponse:
        return self._call("Missions.Launch", self.missions.Launch, missions_pb2.LaunchRequest(build_id=build_id))

    def restart(self) -> missions_pb2.RestartResponse:
        """Reloads the mission DCS runs; the running session (``ended_session``) ends as restarted.

        The engine asks DCS what it runs, then waits up to 30 s for ``net.load_mission``, so the
        call gets at least 45 s.
        """
        return self._call(
            "Missions.Restart", self.missions.Restart, missions_pb2.RestartRequest(), timeout=max(self.timeout, 45.0)
        )

    def results(self, session: str = "") -> missions_pb2.MissionResults:
        return self._call("Missions.GetResults", self.missions.GetResults, missions_pb2.GetResultsRequest(session=session))

    def events(self, session: str = "", kinds: Iterable[int] = (), from_seq: int = 0) -> Iterator[events_pb2.Event]:
        """Live events (a server stream without a deadline; stop it with Ctrl-C)."""
        req = events_pb2.SubscribeEventsRequest(kinds=list(kinds), session=session, from_seq=from_seq)
        try:
            yield from self.events_stub.Subscribe(req, metadata=self.metadata)
        except grpc.RpcError as e:
            if e.code() == grpc.StatusCode.CANCELLED:
                return
            raise self.explain("Events.Subscribe", e) from None
