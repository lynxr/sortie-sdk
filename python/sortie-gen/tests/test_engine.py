"""The engine client and the engine-facing CLI commands against an in-process fake engine."""

from __future__ import annotations

import hashlib
import socket
import threading
from concurrent import futures

import grpc
import pytest

from sortie_gen import cli, engine as enginemod, worker
from sortie_gen._proto.sortie.v1 import (
    common_pb2,
    engine_pb2,
    engine_pb2_grpc,
    events_pb2,
    events_pb2_grpc,
    generator_pb2,
    generator_pb2_grpc,
    missions_pb2,
    missions_pb2_grpc,
    world_pb2,
)
from sortie_gen.errors import EngineError, EngineUnavailable, NotYet

from conftest import OP_NORTH_SCENARIO

ENGINE_BUILD_ID = "01M69CHF2X7EDN6GAQ6HBYH3X2"


class FakeEngine(
    engine_pb2_grpc.EngineServicer,
    missions_pb2_grpc.MissionsServicer,
    generator_pb2_grpc.GeneratorServicer,
    events_pb2_grpc.EventsServicer,
):
    """Just enough of sortied: tokens, BeginBuild, chunked Register, Launch, Restart, results, events."""

    def __init__(self, token: str | None = None, serve_m3: bool = False, restart_error: str | None = None) -> None:
        self.token = token
        self.serve_m3 = serve_m3
        self.restart_error = restart_error
        self.restarts = 0
        self.registered: dict[str, dict] = {}
        self.launched: list[str] = []
        self.chunks: list[int] = []
        self.worker_responses: list[generator_pb2.GenerateResponse] = []

    def _auth(self, context) -> None:
        if self.token is None:
            return
        md = dict(context.invocation_metadata())
        if md.get("authorization") != f"Bearer {self.token}":
            context.abort(grpc.StatusCode.UNAUTHENTICATED, "missing or unknown bearer token")

    def Version(self, request, context):
        self._auth(context)
        return engine_pb2.VersionResponse(engine_version="0.1.0", api="sortie.v1")

    def Health(self, request, context):
        self._auth(context)
        return engine_pb2.HealthResponse(status=engine_pb2.HEALTH_STATUS_OK, session="s1", scenario="op_north", model_time=12)

    def BeginBuild(self, request, context):
        self._auth(context)
        if request.scenario != "op_north" or not request.schema_sha256:
            context.abort(grpc.StatusCode.INVALID_ARGUMENT, "bad BeginBuild")
        return generator_pb2.BeginBuildResponse(build_id=ENGINE_BUILD_ID)

    def ValidateTypes(self, request, context):
        context.abort(grpc.StatusCode.UNIMPLEMENTED, "Generator.ValidateTypes lands in M2")

    def Serve(self, request_iterator, context):
        self._auth(context)
        if not self.serve_m3:
            context.abort(grpc.StatusCode.UNIMPLEMENTED, "the campaign worker loop (Generator.Serve) lands in M3")
        first = next(request_iterator)
        assert first.HasField("hello") and list(first.hello.scenarios) == ["op_north"]
        yield generator_pb2.WorkerDown(request=generator_pb2.GenerateRequest(request_id="r1", scenario="op_north"))
        up = next(request_iterator)
        self.worker_responses.append(up.response)

    def Register(self, request_iterator, context):
        self._auth(context)
        header = None
        data = bytearray()
        for part in request_iterator:
            if part.HasField("header"):
                header = part.header
            else:
                self.chunks.append(len(part.miz_chunk))
                data += part.miz_chunk
        assert header is not None
        if hashlib.sha256(data).hexdigest() != header.miz_sha256 or len(data) != header.miz_size:
            context.abort(grpc.StatusCode.INVALID_ARGUMENT, "miz_sha256 mismatch")
        digest = hashlib.sha256(header.manifest_pb).hexdigest()
        self.registered[header.build_id] = {"file_name": header.file_name, "miz": bytes(data), "digest": digest}
        return missions_pb2.RegisterResponse(
            build_id=header.build_id, path=f"Missions/Sortie/{header.file_name}", digest=digest
        )

    def Launch(self, request, context):
        self._auth(context)
        if request.build_id not in self.registered:
            context.abort(grpc.StatusCode.NOT_FOUND, f"unknown build {request.build_id}")
        self.launched.append(request.build_id)
        return missions_pb2.LaunchResponse(build_id=request.build_id, path=self.registered[request.build_id]["file_name"])

    def Restart(self, request, context):
        self._auth(context)
        if self.restart_error is not None:
            context.abort(grpc.StatusCode.FAILED_PRECONDITION, self.restart_error)
        self.restarts += 1
        # The first restart ends session s1; after it no session runs until DCS reloads.
        return missions_pb2.RestartResponse(ended_session="s1" if self.restarts == 1 else "")

    def GetResults(self, request, context):
        self._auth(context)
        r = missions_pb2.MissionResults(session="s1", scenario="op_north", build_id=ENGINE_BUILD_ID, text="SAM down")
        r.flags["SAM_NORTH_DOWN"] = 1
        r.fates.add(name="SAM North-1", group_ident="SAM_NORTH")
        return r

    def Subscribe(self, request, context):
        self._auth(context)
        yield events_pb2.Event(seq=1, model_time=1.5, session="s1",
                               menu_pressed=events_pb2.MenuPressed(ident="SMOKE_OBJECTIVE", group_name="Blue 1",
                                                                   coalition=common_pb2.COALITION_BLUE))
        yield events_pb2.Event(seq=2, model_time=3.0, session="s1",
                               dead=events_pb2.Dead(initiator=world_pb2.EntitySnap(eid=7, name="SAM North-1")))


@pytest.fixture
def fake_engine(request):
    params = getattr(request, "param", {})
    fake = FakeEngine(**params)
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=4))
    for add in (
        engine_pb2_grpc.add_EngineServicer_to_server,
        missions_pb2_grpc.add_MissionsServicer_to_server,
        generator_pb2_grpc.add_GeneratorServicer_to_server,
        events_pb2_grpc.add_EventsServicer_to_server,
    ):
        add(fake, server)
    port = server.add_insecure_port("127.0.0.1:0")
    server.start()
    fake.address = f"127.0.0.1:{port}"
    yield fake
    server.stop(None)


def free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def test_endpoint_resolution(monkeypatch):
    monkeypatch.delenv(enginemod.ENGINE_ENV, raising=False)
    assert enginemod.resolve_endpoint() == ("127.0.0.1:50777", False)
    assert enginemod.resolve_endpoint("http://127.0.0.1:1/") == ("127.0.0.1:1", False)
    assert enginemod.resolve_endpoint("https://engine.example:443") == ("engine.example:443", True)
    monkeypatch.setenv(enginemod.ENGINE_ENV, "127.0.0.1:9")
    assert enginemod.resolve_endpoint() == ("127.0.0.1:9", False)


def test_engine_down_is_a_friendly_error():
    with enginemod.EngineClient(f"127.0.0.1:{free_port()}", timeout=3) as client:
        with pytest.raises(EngineUnavailable, match=r"Is sortied running\? Start it with `sortied --dev`"):
            client.version()


def test_register_streams_the_miz_in_chunks(fake_engine, op_north):
    client = enginemod.EngineClient(fake_engine.address)
    build_id = client.begin_build(op_north.name, op_north.schema, op_north.schema_sha256(), op_north.generator_string())
    assert build_id == ENGINE_BUILD_ID
    miz = bytes(range(256)) * 9000  # 2.2 MiB: three chunks
    reg = client.register(build_id, b"\x0a\x08op_north", miz, "op_north.miz")
    assert reg.path == "Missions/Sortie/op_north.miz"
    assert fake_engine.chunks == [1024 * 1024, 1024 * 1024, len(miz) - 2 * 1024 * 1024]
    assert fake_engine.registered[build_id]["miz"] == miz


def test_unimplemented_names_the_milestone(fake_engine):
    client = enginemod.EngineClient(fake_engine.address)
    with pytest.raises(NotYet) as e:
        client.validate_types(["Su-27"])
    assert e.value.milestone == "M2" and e.value.exit_code == 2


@pytest.mark.parametrize("fake_engine", [{"token": "s3cret"}], indirect=True)
def test_tokens_are_sent_as_bearer(fake_engine):
    with pytest.raises(EngineError, match="Pass --token or set SORTIE_TOKEN"):
        enginemod.EngineClient(fake_engine.address, token="").version()
    assert enginemod.EngineClient(fake_engine.address, token="s3cret").version().api == "sortie.v1"


def test_cli_register_launch_results_watch(fake_engine, op_north, ui, tmp_path):
    common = ["--engine", fake_engine.address]
    assert cli.scenario_main(op_north, ["register", *common], ui) == 0, ui.stderr
    assert f"registered {ENGINE_BUILD_ID}" in ui.stdout
    reg = fake_engine.registered[ENGINE_BUILD_ID]
    assert reg["file_name"] == f"op_north-{ENGINE_BUILD_ID}.miz"
    # The bootstrap inside the registered .miz carries the engine's build id and digest.
    path = tmp_path / "reg.miz"
    path.write_bytes(reg["miz"])
    from sortie_gen import miz as mizmod

    attach = mizmod.parse_bootstrap(mizmod.find_bootstrap(mizmod.load(path)))
    assert attach == {"scenario": "op_north", "build_id": ENGINE_BUILD_ID, "digest": reg["digest"], "schema": 3}
    assert not (OP_NORTH_SCENARIO.parent / "dist").exists()  # register keeps the build in memory

    assert cli.scenario_main(op_north, ["launch", "--build-id", ENGINE_BUILD_ID, *common], ui) == 0
    assert fake_engine.launched == [ENGINE_BUILD_ID]

    assert cli.scenario_main(op_north, ["results", *common], ui) == 0
    assert "SAM down" in ui.stdout and "flag SAM_NORTH_DOWN = 1" in ui.stdout

    assert cli.scenario_main(op_north, ["watch", *common], ui) == 0
    assert "MENU_PRESSED" in ui.stdout and "ident='SMOKE_OBJECTIVE'" in ui.stdout
    assert "initiator=SAM North-1" in ui.stdout


def test_restart_prints_the_ended_session(fake_engine, op_north, ui):
    common = ["--engine", fake_engine.address]
    assert cli.scenario_main(op_north, ["restart", *common], ui) == 0, ui.stderr
    assert "session s1 ends (restart)" in ui.stdout
    assert "results --session s1" in ui.stdout
    # `sortie-gen restart` needs no scenario file; with no session running it says so.
    assert cli.main(["restart", *common], ui) == 0, ui.stderr
    assert "no session was running" in ui.stdout
    assert fake_engine.restarts == 2


def test_restart_client_returns_the_ended_session(fake_engine):
    with enginemod.EngineClient(fake_engine.address) as client:
        assert client.restart().ended_session == "s1"


_SINGLE_PLAYER = (
    "Missions.Restart needs DCS running as a multiplayer server (net.load_mission is a server API); "
    "in single player restart the mission from the DCS menu"
)


@pytest.mark.parametrize("fake_engine", [{"restart_error": _SINGLE_PLAYER}], indirect=True)
def test_restart_refusal_reaches_the_user(fake_engine, op_north, ui):
    code = cli.scenario_main(op_north, ["restart", "--engine", fake_engine.address], ui)
    assert code == 1
    assert ui.stderr == f"error: Missions.Restart failed: FAILED_PRECONDITION: {_SINGLE_PLAYER}\n"
    assert fake_engine.restarts == 0


def test_cli_engine_down_exit_code(op_north, ui):
    code = cli.scenario_main(op_north, ["results", "--engine", f"127.0.0.1:{free_port()}", "--timeout", "3"], ui)
    assert code == 1
    assert ui.stderr.startswith("error: cannot reach the Sortie engine")


def test_worker_reports_m3(fake_engine, tmp_path, ui):
    campaign = tmp_path / "campaign.py"
    campaign.write_text("SCENARIOS = ['op_north']\ndef next_turn(turn):\n    raise RuntimeError('unused')\n")
    code = cli.main(["worker", str(campaign), "--once", "--engine", fake_engine.address], ui)
    assert code == 2
    assert "lands in M3" in ui.stderr


@pytest.mark.filterwarnings("ignore::sortie_gen.scenario.SortieWarning")
@pytest.mark.parametrize("fake_engine", [{"serve_m3": True}], indirect=True)
def test_worker_answers_a_turn(fake_engine, op_north):
    """The worker side of the M3 loop already works against an engine that speaks it."""
    turns = []

    def next_turn(turn):
        turns.append(turn)
        return op_north

    client = enginemod.EngineClient(fake_engine.address)
    t = threading.Thread(target=worker.serve, args=(client, next_turn, ["op_north"]), kwargs={"log": lambda _m: None})
    t.start()
    t.join(30)
    assert not t.is_alive()
    assert [x.request_id for x in turns] == ["r1"]
    [resp] = fake_engine.worker_responses
    assert resp.request_id == "r1" and resp.build_id == ENGINE_BUILD_ID
    assert ENGINE_BUILD_ID in fake_engine.registered
