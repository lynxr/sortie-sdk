"""Player slots can be taken in multiplayer: the airfield a player flight starts from belongs to
the flight's coalition.

DCS's multiplayer slot dialog (``MultiplayerSelectRoleDialog.lua`` in
``Scripts/UI/MultiplayerSelectRoleMap``, ``isSlotAvailable``) refuses a parking slot when the
airfield's coalition is not the player's. op_north used to leave Senaki-Kolkhi neutral: in single player Blue 1 flew, in
multiplayer its slots could not be taken and the host stayed a spectator.
"""

from __future__ import annotations

import io
import warnings
import zipfile

import pytest
from dcs import planes
from dcs.terrain import caucasus

from sortie_gen import Coalition, Scenario, StartType
from sortie_gen.scenario import SortieWarning
from sortie_gen.errors import ScenarioError
from sortie_gen._proto.sortie.v1 import common_pb2

GROUND_STARTS = ("TakeOffParking", "TakeOffParkingHot", "TakeOff")
SIDE_OF = {"blue": "BLUE", "red": "RED", "neutrals": "NEUTRAL"}
PROTO_SIDE = {
    common_pb2.COALITION_BLUE: "BLUE",
    common_pb2.COALITION_RED: "RED",
    common_pb2.COALITION_NEUTRAL: "NEUTRAL",
}


def scenario() -> Scenario:
    return Scenario("test_sc", schema=1, theatre=caucasus.Caucasus(), start="2026-11-01T06:30:00Z", source=None)


def build(sc: Scenario):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return sc.build()


def miz_tables(miz: bytes):
    lupa = pytest.importorskip("lupa.lua51")
    with zipfile.ZipFile(io.BytesIO(miz)) as z:
        mission = z.read("mission").decode("utf-8")
        warehouses = z.read("warehouses").decode("utf-8")
    lua = lupa.LuaRuntime(unpack_returned_tuples=True)
    lua.execute(mission)
    lua.execute(warehouses)
    return lua.eval("mission"), lua.eval("warehouses")


def player_starts(mission) -> list[tuple[str, str, int]]:
    """``(group, coalition side key, airdromeId)`` of every Client/Player group starting on the ground."""
    out = []
    for side in ("red", "blue", "neutrals"):
        col = mission.coalition[side]
        if col is None:
            continue
        for _, country in (col.country or {}).items():
            for cat in ("plane", "helicopter"):
                if country[cat] is None:
                    continue
                for _, g in country[cat].group.items():
                    skills = {u.skill for _, u in g.units.items()}
                    wp = g.route.points[1]
                    if skills & {"Client", "Player"} and wp.type in GROUND_STARTS:
                        out.append((g.name, side, int(wp.airdromeId)))
    return out


def test_op_north_player_airfield_belongs_to_blue(op_north_build):
    mission, warehouses = miz_tables(op_north_build.miz)
    starts = player_starts(mission)
    assert starts == [("Blue 1", "blue", 23)]  # Senaki-Kolkhi
    for group, side, aid in starts:
        assert warehouses.airports[aid].coalition == SIDE_OF[side], group


def test_manifest_airbases_match_the_miz(op_north_build):
    _, warehouses = miz_tables(op_north_build.miz)
    for a in op_north_build.manifest.airbases:
        assert PROTO_SIDE[a.coalition] == warehouses.airports[a.airbase_id].coalition, a.ident


def test_a_declared_airbase_without_owner_takes_the_flight_coalition():
    sc = scenario()
    sc.airbase("Senaki-Kolkhi", ident="SENAKI")
    sc.player_flight("Viper", planes.F_16C_50, country="USA", airbase="Senaki-Kolkhi", start=StartType.Hot)
    b = build(sc)
    _, warehouses = miz_tables(b.miz)
    [senaki] = b.manifest.airbases
    assert warehouses.airports[senaki.airbase_id].coalition == "BLUE"
    assert senaki.coalition == common_pb2.COALITION_BLUE


def test_red_player_flight_gives_its_airfield_to_red():
    sc = scenario()
    sc.player_flight("Flanker", planes.Su_27, country="Russia", airbase="Maykop-Khanskaya")
    mission, warehouses = miz_tables(build(sc).miz)
    [(_, side, aid)] = player_starts(mission)
    assert (side, warehouses.airports[aid].coalition) == ("red", "RED")


def test_a_runway_start_owns_its_airfield_too():
    sc = scenario()
    sc.player_flight("Viper", planes.F_16C_50, country="USA", airbase="Kutaisi", start=StartType.Runway)
    mission, warehouses = miz_tables(build(sc).miz)
    [(_, side, aid)] = player_starts(mission)
    assert (side, warehouses.airports[aid].coalition) == ("blue", "BLUE")


def test_a_hook_that_places_players_at_a_foreign_airfield_gets_a_warning():
    import dcs

    sc = scenario()

    @sc.pydcs
    def add_slot(m):
        g = m.flight_group_from_airport(
            m.country("USA"), "Hooked", planes.F_16C_50, m.terrain.airports["Batumi"],
            start_type=dcs.mission.StartType.Cold, group_size=1,
        )
        g.units[0].skill = dcs.unit.Skill.Client

    with pytest.warns(SortieWarning, match="Hooked.*Batumi.*multiplayer"):
        sc.build()


def test_an_air_start_leaves_the_airfield_alone():
    sc = scenario()
    sc.player_flight(
        "Viper", planes.F_16C_50, country="USA", airbase="Senaki-Kolkhi", start=StartType.Air,
        at=sc.point(-281000, 646000),
    )
    _, warehouses = miz_tables(build(sc).miz)
    senaki = caucasus.Caucasus().airports["Senaki-Kolkhi"]
    assert warehouses.airports[int(senaki.id)].coalition == "NEUTRAL"


def test_an_airfield_declared_for_another_coalition_is_refused():
    sc = scenario()
    sc.airbase("Senaki-Kolkhi", ident="SENAKI", coalition=Coalition.Red)
    sc.player_flight("Viper", planes.F_16C_50, country="USA", airbase="Senaki-Kolkhi")
    with pytest.raises(ScenarioError, match="multiplayer") as e:
        build(sc)
    assert "Viper" in str(e.value) and "gives to red" in str(e.value)


def test_players_of_two_coalitions_on_one_airfield_are_refused():
    sc = scenario()
    sc.player_flight("Viper", planes.F_16C_50, country="USA", airbase="Senaki-Kolkhi")
    sc.player_flight("Flanker", planes.Su_27, country="Russia", airbase="Senaki-Kolkhi")
    with pytest.raises(ScenarioError, match="multiplayer") as e:
        build(sc)
    assert "Viper" in str(e.value) and "Flanker" in str(e.value)
