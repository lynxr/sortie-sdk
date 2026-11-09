"""``sortie-gen worker campaign.py``: the campaign worker on ``Generator.Serve`` (M3).

A campaign module defines::

    SCENARIOS = ["caucasus_campaign"]          # optional; default: the scenario of next_turn()

    def next_turn(turn: Turn) -> Scenario:     # build the next turn from the last results
        ...

The worker connects to the engine, says which scenarios it serves, and for every
``GenerateRequest`` calls ``next_turn``, registers the build (``BeginBuild`` + ``Register``)
and answers with its build id; the engine then launches it. The engine side of this loop
lands in M3: until then the engine answers ``UNIMPLEMENTED`` and the worker says so and exits.
"""

from __future__ import annotations

import json
import os
import queue
import runpy
import socket
import time
import traceback
from collections.abc import Callable, Iterator
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import grpc

from .engine import EngineClient
from .errors import EngineUnavailable, GeneratorError, NotYet
from ._proto.sortie.v1 import generator_pb2
from ._proto.sortie.v1 import missions_pb2


@dataclass
class Turn:
    """What the engine knows when it asks for the next turn."""

    request_id: str
    scenario: str
    state: dict[str, Any] = field(default_factory=dict)
    results: missions_pb2.MissionResults | None = None
    params: dict[str, str] = field(default_factory=dict)

    @classmethod
    def from_request(cls, r: generator_pb2.GenerateRequest) -> Turn:
        state: dict[str, Any] = {}
        for doc in r.state:
            try:
                state[doc.name] = json.loads(doc.json) if doc.json else None
            except json.JSONDecodeError:
                state[doc.name] = doc.json
        return cls(
            request_id=r.request_id,
            scenario=r.scenario,
            state=state,
            results=r.results if r.HasField("results") else None,
            params=dict(r.params),
        )


def load_campaign(path: str | Path) -> tuple[Callable[[Turn], Any], list[str]]:
    module = runpy.run_path(str(path), run_name="__sortie_worker__")
    fn = module.get("next_turn")
    if not callable(fn):
        raise GeneratorError(f"{path} must define next_turn(turn) -> Scenario")
    scenarios = list(module.get("SCENARIOS") or [])
    return fn, scenarios


def register_turn(client: EngineClient, sc: Any) -> str:
    """BeginBuild + build + Register; returns the registered build id."""
    build_id = client.begin_build(sc.name, sc.schema, sc.schema_sha256(), sc.generator_string())
    b = sc.build(build_id)
    client.register(b.build_id, b.manifest_pb, b.miz, f"{sc.name}-{b.build_id}.miz")
    return b.build_id


def answer(client: EngineClient, next_turn: Callable[[Turn], Any], req: generator_pb2.GenerateRequest) -> generator_pb2.GenerateResponse:
    try:
        sc = next_turn(Turn.from_request(req))
        build_id = register_turn(client, sc)
        return generator_pb2.GenerateResponse(request_id=req.request_id, build_id=build_id)
    except Exception as e:  # the engine gets the error instead of a dead worker
        return generator_pb2.GenerateResponse(
            request_id=req.request_id,
            error=generator_pb2.GenerateError(message=str(e), traceback=traceback.format_exc()),
        )


def serve(
    client: EngineClient,
    next_turn: Callable[[Turn], Any],
    scenarios: list[str],
    *,
    worker: str | None = None,
    version: str = "",
    log: Callable[[str], None] = print,
) -> None:
    """One ``Generator.Serve`` stream until the engine closes it."""
    outbox: queue.Queue[generator_pb2.WorkerUp | None] = queue.Queue()
    hello = generator_pb2.WorkerHello(
        scenarios=scenarios, worker=worker or f"{socket.gethostname()}:{os.getpid()}", version=version
    )

    def up() -> Iterator[generator_pb2.WorkerUp]:
        yield generator_pb2.WorkerUp(hello=hello)
        while True:
            item = outbox.get()
            if item is None:
                return
            yield item

    try:
        for down in client.generator.Serve(up(), metadata=client.metadata):
            if down.HasField("request"):
                log(f"turn requested for {down.request.scenario} ({down.request.request_id})")
                resp = answer(client, next_turn, down.request)
                if resp.HasField("error"):
                    log(f"next_turn failed: {resp.error.message}")
                else:
                    log(f"registered build {resp.build_id}")
                outbox.put(generator_pb2.WorkerUp(response=resp))
    except grpc.RpcError as e:
        raise client.explain("Generator.Serve", e) from None
    finally:
        outbox.put(None)


def run(path: str | Path, client: EngineClient, *, once: bool = False, log: Callable[[str], None] = print) -> int:
    from . import __version__

    next_turn, scenarios = load_campaign(path)
    backoff = 1.0
    while True:
        try:
            log(f"worker for {', '.join(scenarios) or Path(path).name}: connecting to {client.address}")
            serve(client, next_turn, scenarios, version=f"sortie-gen {__version__}", log=log)
            log("the engine closed the worker stream")
            backoff = 1.0
        except NotYet:
            raise
        except EngineUnavailable as e:
            if once:
                raise
            log(f"{e} (retrying in {backoff:.0f} s)")
            time.sleep(backoff)
            backoff = min(backoff * 2, 30.0)
            continue
        if once:
            return 0
