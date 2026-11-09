"""The type check before register/launch (``Generator.ValidateTypes``) against a fake engine."""

from __future__ import annotations

import warnings
from concurrent import futures

import dcs
import grpc
import pytest
from dcs import planes, vehicles
from dcs.terrain import caucasus

from sortie_gen import Scenario, cli, engine as enginemod
from sortie_gen._proto.sortie.v1 import (
    common_pb2,
    generator_pb2,
    generator_pb2_grpc,
    missions_pb2_grpc,
)
from sortie_gen.errors import EngineNotReady, NotYet

from test_engine import ENGINE_BUILD_ID, FakeEngine

GROUND = common_pb2.CATEGORY_GROUND_UNIT
AIR = common_pb2.CATEGORY_AIRPLANE
STATIC = common_pb2.CATEGORY_STATIC

OP_NORTH_TYPES = ["F-16C_50", "S-300PS 40B6M tr", "S-300PS 54K6 cp", "S-300PS 5P85C ln", "S-300PS 64H6E sr", "Su-27"]
# What the mission running in the fake DCS knows.
KNOWN = {
    **{t: GROUND for t in OP_NORTH_TYPES if t.startswith("S-300")},
    "Su-27": AIR,
    "F-16C_50": AIR,
    "T-72B": GROUND,
    "FARP": STATIC,
    ".Command Center": STATIC,
}
NO_MISSION = (
    "no mission is running in DCS: ValidateTypes asks the live mission (Unit.getDescByName); "
    "start any mission, e.g. missions/sortie-lobby.miz"
)


class TypesEngine(FakeEngine):
    """The fake engine with M2's ``Generator.ValidateTypes`` (the base one predates it)."""

    def __init__(self, known: dict[str, int] | None = None, answer: str = "ok") -> None:
        super().__init__()
        self.known = KNOWN if known is None else known
        self.answer = answer
        self.asked: list[list[str]] = []

    def ValidateTypes(self, request, context):
        self._auth(context)
        names = list(request.type_names)
        self.asked.append(names)
        if self.answer == "unimplemented":
            context.abort(grpc.StatusCode.UNIMPLEMENTED, "Generator.ValidateTypes lands in M2")
        if self.answer == "no-mission":
            context.abort(grpc.StatusCode.FAILED_PRECONDITION, NO_MISSION)
        if self.answer == "lua-error":
            context.abort(grpc.StatusCode.ABORTED, 'Unit.getDescByName("Su-27") failed in DCS (LUA_ERROR): boom')
        if not names or len(names) > 512:
            context.abort(grpc.StatusCode.INVALID_ARGUMENT, f"{len(names)} type names")
        return generator_pb2.ValidateTypesResponse(
            results=[
                generator_pb2.TypeCheck(type_name=n, known=n in self.known, category=self.known.get(n, 0))
                for n in names
            ]
        )


@pytest.fixture
def types_engine(request):
    fake = TypesEngine(**getattr(request, "param", {}))
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=4))
    generator_pb2_grpc.add_GeneratorServicer_to_server(fake, server)
    missions_pb2_grpc.add_MissionsServicer_to_server(fake, server)
    port = server.add_insecure_port("127.0.0.1:0")
    server.start()
    fake.address = f"127.0.0.1:{port}"
    yield fake
    server.stop(None)


def run(sc, argv, fake, ui) -> int:
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return cli.scenario_main(sc, [*argv, "--engine", fake.address], ui)


def test_register_checks_every_type_first(types_engine, op_north, ui):
    assert run(op_north, ["register"], types_engine, ui) == 0, ui.stderr
    assert types_engine.asked == [OP_NORTH_TYPES]
    assert "checked 6 DCS type(s) with the mission running in DCS: 6 known" in ui.stdout
    assert ENGINE_BUILD_ID in types_engine.registered


@pytest.mark.parametrize("types_engine", [{"known": {k: v for k, v in KNOWN.items() if k != "S-300PS 64H6E sr"}}], indirect=True)
def test_unknown_types_stop_register(types_engine, op_north, ui):
    assert run(op_north, ["register"], types_engine, ui) == 1
    assert "\nerror: DCS on the server does not know 1 type(s) this scenario uses:\n" in ui.stderr
    assert "  'S-300PS 64H6E sr' (group 'SAM North')\n" in ui.stderr
    assert "module or mod that provides it not installed on the DCS server" in ui.stderr
    assert "--no-validate skips this check" in ui.stderr
    assert types_engine.registered == {}


@pytest.mark.parametrize("types_engine", [{"known": {}}], indirect=True)
def test_launch_checks_types_too_and_no_validate_skips(types_engine, op_north, ui):
    assert run(op_north, ["launch"], types_engine, ui) == 1
    assert "does not know 6 type(s)" in ui.stderr
    assert "  'F-16C_50' (player flight 'Blue 1')\n" in ui.stderr
    assert "  'Su-27' (template 'Red CAP')\n" in ui.stderr
    assert types_engine.launched == []
    asked = len(types_engine.asked)
    assert run(op_north, ["launch", "--no-validate"], types_engine, ui) == 0, ui.stderr
    assert types_engine.launched == [ENGINE_BUILD_ID]
    assert len(types_engine.asked) == asked, "--no-validate never asks"
    # A registered build is not built again, so there is nothing to check.
    assert run(op_north, ["launch", "--build-id", ENGINE_BUILD_ID], types_engine, ui) == 0
    assert len(types_engine.asked) == asked


@pytest.mark.parametrize(
    ("types_engine", "said"),
    [
        ({"answer": "no-mission"}, f"warning: unit and static types not checked: {NO_MISSION} (registering anyway)"),
        ({"answer": "unimplemented"}, "warning: unit and static types not checked: this engine cannot check them yet"),
    ],
    indirect=["types_engine"],
)
def test_when_the_check_cannot_run_register_warns_and_goes_on(types_engine, said, op_north, ui):
    assert run(op_north, ["register"], types_engine, ui) == 0, ui.stderr
    assert said in ui.stderr
    assert ENGINE_BUILD_ID in types_engine.registered


@pytest.mark.parametrize("types_engine", [{"answer": "lua-error"}], indirect=True)
def test_a_failed_check_stops_with_the_engines_reason(types_engine, op_north, ui):
    assert run(op_north, ["register"], types_engine, ui) == 1
    assert 'Unit.getDescByName("Su-27") failed in DCS (LUA_ERROR): boom' in ui.stderr
    assert "(--no-validate skips the type check)" in ui.stderr
    assert types_engine.registered == {}


def with_statics(*, static_type=None) -> Scenario:
    sc = Scenario("statics", schema=1, theatre=caucasus.Caucasus(), start="2026-11-01T06:30:00Z", source=None)
    sc.ground_group("Tanks", country="Russia", at=sc.point(0, 0), units=[vehicles.Armor.T_72B])
    sc.player_flight("Viper", planes.F_16C_50, country="USA", airbase="Senaki-Kolkhi")

    @sc.pydcs
    def add_statics(m: dcs.Mission) -> None:
        russia = m.country("Russia")
        m.farp(russia, "FARP London", dcs.mapping.Point(1000, 1000, m.terrain))
        m.static_group(russia, "HQ", static_type or dcs.statics.Fortification._Command_Center,
                       dcs.mapping.Point(2000, 1000, m.terrain))

    return sc


def test_builds_list_unit_and_static_type_uses():
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        b = with_statics().build()
    uses = {(u.type_name, u.where, u.static) for u in b.type_uses}
    assert uses == {
        ("T-72B", "group 'Tanks'", False),
        ("F-16C_50", "player flight 'Viper'", False),
        ("FARP", "static 'FARP London'", True),
        (".Command Center", "static 'HQ'", True),
    }
    assert b.type_names() == [".Command Center", "F-16C_50", "FARP", "T-72B"]


@pytest.mark.parametrize("types_engine", [{"known": {**KNOWN, "T-72B": STATIC}}], indirect=True)
def test_validate_prints_the_table_and_flags_a_unit_known_only_as_static(types_engine, ui):
    assert run(with_statics(), ["validate"], types_engine, ui) == 0, ui.stderr
    lines = ui.stdout.splitlines()
    assert lines[0].split() == ["type", "known", "category", "used", "by"]
    rows = {line.split()[0]: line for line in lines[1:-1]}
    assert rows["'FARP'"].split()[1:4] == ["yes", "static", "static"]
    assert rows["'F-16C_50'"].split()[1:3] == ["yes", "airplane"]
    assert lines[-1] == "4 of 4 type(s) are known to the mission running in DCS"
    assert "warning: 'T-72B' is a unit of group 'Tanks', but DCS knows it only as a static object type" in ui.stderr


@pytest.mark.parametrize("types_engine", [{"answer": "no-mission"}], indirect=True)
def test_validate_fails_when_the_check_cannot_run(types_engine, ui):
    assert run(with_statics(), ["validate"], types_engine, ui) == 1
    assert ui.stderr.strip().endswith(NO_MISSION)


def test_validate_reports_unknown_types(types_engine, ui):
    sc = with_statics(static_type=type("Typo", (dcs.statics.Fortification._Command_Center,), {"id": ".Comand Center"}))
    assert run(sc, ["validate"], types_engine, ui) == 1
    row = next(line for line in ui.stdout.splitlines() if line.startswith("'.Comand Center'"))
    assert row.removeprefix("'.Comand Center'").split()[:2] == ["NO", "-"]
    assert "error: DCS on the server does not know 1 type(s)" in ui.stderr
    assert "  '.Comand Center' (static 'HQ')" in ui.stderr


def test_the_client_splits_long_lists_and_maps_failed_precondition(types_engine):
    client = enginemod.EngineClient(types_engine.address)
    names = [f"T{k:04}" for k in range(600)] + ["Su-27", "Su-27"]
    r = client.validate_types(names)
    assert [len(a) for a in types_engine.asked] == [512, 89]
    assert len(r.results) == 601
    assert [c.type_name for c in r.results if c.known] == ["Su-27"]
    types_engine.answer = "no-mission"
    with pytest.raises(EngineNotReady) as e:
        client.validate_types(["Su-27"])
    assert e.value.details == NO_MISSION
    types_engine.answer = "unimplemented"
    with pytest.raises(NotYet):
        client.validate_types(["Su-27"])
