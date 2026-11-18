"""The universal call from Python: the generated wrappers (``sortie_gen.dcs_api``) build the
``DcsCall`` the engine checks against the catalog, ``EngineClient.dcs_call`` submits it and
decodes the typed result. Against an in-process fake ``Commands`` service."""

from __future__ import annotations

import ast
import tomllib
from concurrent import futures
from pathlib import Path

import grpc
import pytest

from sortie_gen import dcs as dcsmod
from sortie_gen.dcs_api import Dcs
from sortie_gen.engine import EngineClient
from sortie_gen.errors import CommandFailed, ResultNotKept
from sortie_gen._proto.sortie.v1 import commands_pb2, commands_pb2_grpc, common_pb2

ROOT = Path(__file__).resolve().parents[3]


def num(x: float) -> commands_pb2.DcsValue:
    return commands_pb2.DcsValue(number=x)


def table(pairs: list[tuple[commands_pb2.DcsValue, commands_pb2.DcsValue]]) -> commands_pb2.DcsValue:
    return commands_pb2.DcsValue(
        table=commands_pb2.DcsValueTable(pairs=[commands_pb2.DcsValuePair(key=k, value=v) for k, v in pairs])
    )


def s(x: str) -> commands_pb2.DcsValue:
    return commands_pb2.DcsValue(string=x)


class FakeCommands(commands_pb2_grpc.CommandsServicer):
    """Answers each DcsCall by path with a scripted DcsReturn (or error) and keeps the commands."""

    def __init__(self) -> None:
        self.commands: list[commands_pb2.Command] = []

    def Submit(self, request, context):
        self.commands.append(request)
        call = request.dcs_call
        r = commands_pb2.CommandResult(id="api-1")
        if request.key == "not-kept":
            # A keyed repeat whose stored result was above the key store's size limit.
            r.replayed = True
            r.ok.SetInParent()
        elif call.path == "land.getSurfaceType":
            r.reply.call.values.append(commands_pb2.DcsValue(enum_key="ROAD"))
            r.ok.string = "ROAD"
        elif call.path == "Group.getUnits":
            r.reply.call.values.append(table([(num(1), commands_pb2.DcsValue(ref=0))]))
            r.reply.call.refs.append(
                commands_pb2.ObjectRef(
                    object_class=commands_pb2.OBJECT_CLASS_UNIT, eid=42, name="SAM North-1", type_name="T-72B"
                )
            )
        elif call.path == "Object.getPoint":
            r.reply.call.values.append(table([(s("x"), num(1)), (s("y"), num(2)), (s("z"), num(3))]))
        elif call.path == "CoalitionObject.getCoalition":
            r.reply.call.values.append(commands_pb2.DcsValue(enum_key="BLUE"))
        elif call.path == "atmosphere.getTemperatureAndPressure":
            r.reply.call.values.extend([num(288.15), num(101325)])
        elif call.path == "Spot.createLaser":
            r.reply.call.values.append(commands_pb2.DcsValue(ref=0))
            r.reply.call.refs.append(commands_pb2.ObjectRef(object_class=commands_pb2.OBJECT_CLASS_SPOT, handle=8))
        elif call.path == "Airbase.setCoalition":
            r.err.code = common_pb2.ERROR_CODE_REJECTED
            r.err.message = "`Airbase.setCoalition` is switched off"
            r.err.capability = "call:mission:Airbase.setCoalition"
        elif call.path == "Unit.getAmmo":
            r.reply.call.values.append(commands_pb2.DcsValue(cut=3))
            r.reply.call.truncated = True
        else:
            r.reply.call.SetInParent()
        return r


@pytest.fixture
def client():
    fake = FakeCommands()
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=2))
    commands_pb2_grpc.add_CommandsServicer_to_server(fake, server)
    port = server.add_insecure_port("127.0.0.1:0")
    server.start()
    with EngineClient(f"127.0.0.1:{port}", timeout=5) as c:
        c.fake = fake
        yield c
    server.stop(None)


def test_reads_build_the_catalog_call_and_decode_typed_results(client):
    dcs = Dcs(client)
    assert dcs.land().surface_type((1000.0, 2000.0)) == "ROAD"
    cmd = client.fake.commands[-1]
    assert cmd.min_api_rev == 1 and cmd.api_rev == 1 and not cmd.key
    call = cmd.dcs_call
    assert (call.path, call.env) == ("land.getSurfaceType", common_pb2.ENV_MISSION)
    assert call.args[0].vec2.north == 1000.0 and call.args[0].vec2.east == 2000.0

    units = dcs.group("SAM North").units()
    assert units[0] == dcsmod.DcsObject("unit", 42, "SAM North-1", "T-72B", 0)
    assert client.fake.commands[-1].dcs_call.receiver.group.name == "SAM North"

    # A returned object names itself again; points come back as tuples, coalitions as names.
    assert dcs.unit(units[0]).point() == (1.0, 2.0, 3.0)
    assert client.fake.commands[-1].dcs_call.receiver.object.eid == 42
    assert dcs.airbase("Kutaisi").coalition() == "blue"
    assert client.fake.commands[-1].dcs_call.receiver.airbase.name == "Kutaisi"
    assert dcs.atmosphere().temperature_and_pressure((0, 1000, 0)) == (288.15, 101325.0)
    assert dcs.unit("Viper-1").ammo() == dcsmod.Cut(3)


def test_arguments_of_every_kind(client):
    dcs = Dcs(client)
    dcs.effects().signal_flare((1, 2, 3), "Red", 0.5)
    args = client.fake.commands[-1].dcs_call.args
    assert args[0].vec3.z == 3 and args[1].enum_key == "trigger.flareColor.Red" and args[2].value.number == 0.5
    # land.profile takes two Vec3 (DCS reads x and z), not map points.
    dcs.land().profile((1000, 0, 2000), (3000, 0, 6000))
    ends = client.fake.commands[-1].dcs_call.args
    assert [a.WhichOneof("kind") for a in ends] == ["vec3", "vec3"]
    assert (ends[1].vec3.x, ends[1].vec3.z) == (3000, 6000)
    dcs.sounds().out_sound_for_group(41, "l10n/DEFAULT/beep.ogg", key="beep-1")
    cmd = client.fake.commands[-1]
    assert cmd.key == "beep-1"
    assert cmd.dcs_call.args[0].id_of.group.eid == 41
    dcs.sounds().out_sound_for_coalition("blue", "a.ogg")
    assert client.fake.commands[-1].dcs_call.args[0].coalition == common_pb2.COALITION_BLUE
    # A middle optional argument is sent as nil, trailing ones are left out.
    handle = dcs.spots().create_laser("JTAC-1", None, (1, 2, 3))
    assert handle == 8
    laser = client.fake.commands[-1].dcs_call
    assert len(laser.args) == 3
    assert laser.args[0].object.object.unit_name == "JTAC-1"
    assert laser.args[1].value.WhichOneof("kind") == "nil"
    dcs.spot(handle).set_code(1688)
    assert client.fake.commands[-1].dcs_call.receiver.handle == 8
    dcs.warehouse("Kutaisi").add_item("weapons.missiles.AIM_120C", 8)
    wh = client.fake.commands[-1].dcs_call.receiver.warehouse
    assert wh.airbase.name == "Kutaisi"
    # A warehouse item by its wsType goes as the table {4, 4, 7, 22}.
    dcs.warehouse("Kutaisi").set_item((4, 4, 7, 22), 20)
    item = client.fake.commands[-1].dcs_call.args[0].value.table
    assert [(p.key.number, p.value.number) for p in item.pairs] == [(1, 4), (2, 4), (3, 7), (4, 22)]
    for bad in [(4, 4, 7), (4, 4, 7, -1), (4, 4, 7.5, 22), (True, 4, 7, 22)]:
        with pytest.raises(ValueError, match="wsType"):
            dcs.warehouse("Kutaisi").item_count(bad)
    assert dcsmod.arg_value({"a": [1, 2], "b": True}).value.table.pairs[0].key.string == "a"
    with pytest.raises(ValueError):
        dcsmod.coalition("purple")


def test_a_refused_call_raises_with_its_capability(client):
    with pytest.raises(CommandFailed) as e:
        Dcs(client).airbase("Kutaisi").set_coalition("blue", key="k")
    assert e.value.code == "REJECTED"
    assert e.value.capability == "call:mission:Airbase.setCoalition"


def test_a_keyed_repeat_without_its_values_is_a_distinct_error(client):
    """The engine answers a keyed repeat from its key store; a result above 64 KiB is stored
    without its values, and the client says so instead of returning nothing."""
    with pytest.raises(ResultNotKept) as e:
        Dcs(client).spots().create_laser("JTAC-1", None, (1, 2, 3), key="not-kept")
    assert "64 KiB" in str(e.value) and "Spot.createLaser" in str(e.value)
    # A repeat that carries its typed result decodes as the first answer did.
    assert Dcs(client).spots().create_laser("JTAC-1", None, (1, 2, 3), key="lase-1") == 8


def test_the_generated_module_is_valid_and_lists_every_entry():
    """The module is checked against the catalog by `cargo xtask catalog-gen --check`; here
    only that it parses and covers every [[call]] path of the catalog."""
    module = Path(dcsmod.__file__).with_name("dcs_api.py").read_text(encoding="utf-8")
    ast.parse(module)
    catalog = tomllib.loads((ROOT / "catalog" / "dcs_api.toml").read_text(encoding="utf-8"))
    paths = {c["path"] for c in catalog["call"]}
    assert len(paths) > 100
    for p in paths:
        assert f'"{p}"' in module, p
