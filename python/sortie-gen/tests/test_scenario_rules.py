"""Names, idents and roles: what the Scenario wrapper refuses before pydcs ever sees it."""

from __future__ import annotations

import warnings

import pytest
from dcs import planes, vehicles
from dcs.terrain import caucasus

from sortie_gen import Coalition, Scenario, StartType
from sortie_gen.errors import InvalidName, NameConflict, ScenarioError
from sortie_gen.scenario import parse_duration, parse_start

SA = vehicles.AirDefence.S_300PS_5P85C_ln


def scenario(**kw) -> Scenario:
    kw.setdefault("schema", 1)
    kw.setdefault("start", "2026-11-01T06:30:00Z")
    return Scenario(kw.pop("name", "test_sc"), theatre=caucasus.Caucasus(), source=None, **kw)


@pytest.fixture
def sc() -> Scenario:
    return scenario()


@pytest.mark.parametrize("name", ["Op_North", "1op", "op-north", "", "op north"])
def test_scenario_name_rule(name):
    with pytest.raises(InvalidName):
        scenario(name=name)


@pytest.mark.parametrize("ident", ["sam_north", "SAM-NORTH", "1SAM", "SAM NORTH", ""])
def test_ident_rule(sc, ident):
    with pytest.raises(InvalidName):
        sc.zone("Zone", ident=ident, at=sc.point(0, 0), radius=10)


@pytest.mark.parametrize("role", ["SAM", "sa m", "-sam", ""])
def test_role_rule(sc, role):
    with pytest.raises(InvalidName):
        sc.ground_group("G", ident="G", country="Russia", at=sc.point(0, 0), units=[SA], roles={role})


@pytest.mark.parametrize("name", ["Red#1", " Red", "Red ", "Red\tCAP", "x" * 129, ""])
def test_dcs_group_name_rule(sc, name):
    with pytest.raises(InvalidName):
        sc.ground_group(name, country="Russia", at=sc.point(0, 0), units=[SA])


def test_group_names_are_unique_across_groups_templates_and_flights(sc):
    sc.ground_group("Alpha", country="Russia", at=sc.point(0, 0), units=[SA])
    with pytest.raises(NameConflict, match="already exists"):
        sc.ground_group("Alpha", country="Russia", at=sc.point(100, 0), units=[SA])
    with pytest.raises(NameConflict):
        sc.air_template("Alpha", planes.Su_27, ident="ALPHA", country="Russia")
    with pytest.raises(NameConflict):
        sc.player_flight("Alpha", planes.F_16C_50, country="USA", airbase="Senaki-Kolkhi")


def test_templates_share_the_group_namespace(sc):
    sc.air_template("Bravo", planes.Su_27, ident="BRAVO", country="Russia")
    with pytest.raises(NameConflict, match="template"):
        sc.ground_group("Bravo", country="Russia", at=sc.point(0, 0), units=[SA])


def test_group_and_unit_namespaces_are_separate(sc):
    # Units are "<group>-<k>": group "Alpha-1" (unit "Alpha-1-1") does not clash with unit "Alpha-1".
    sc.ground_group("Alpha", country="Russia", at=sc.point(0, 0), units=[SA, SA])
    g = sc.ground_group("Alpha-1", country="Russia", at=sc.point(500, 0), units=[SA])
    assert g.units == ("Alpha-1-1",)


def test_idents_are_unique_per_kind(sc):
    sc.zone("One", ident="OBJ", at=sc.point(0, 0), radius=10)
    with pytest.raises(NameConflict, match="declared twice"):
        sc.zone("Two", ident="OBJ", at=sc.point(0, 0), radius=10)
    sc.flag("DOWN")
    with pytest.raises(NameConflict):
        sc.flag("DOWN")
    sc.menu("SMOKE", "Smoke")
    with pytest.raises(NameConflict):
        sc.menu("SMOKE", "Smoke again")
    sc.airbase("Maykop-Khanskaya", ident="MAYKOP")
    with pytest.raises(NameConflict):
        sc.airbase("Maykop-Khanskaya", ident="MAYKOP_2")


def test_zone_names_are_unique(sc):
    sc.zone("Objective", ident="A", at=sc.point(0, 0), radius=10)
    with pytest.raises(NameConflict):
        sc.zone("Objective", ident="B", at=sc.point(0, 0), radius=10)


def test_templates_need_an_ident(sc):
    with pytest.raises(ScenarioError, match="needs an ident"):
        sc.air_template("Red CAP", planes.Su_27, ident=None, country="Russia")


def test_bad_inputs_have_readable_errors(sc):
    with pytest.raises(ScenarioError, match="unknown country"):
        sc.ground_group("G", country="Atlantis", at=sc.point(0, 0), units=[SA])
    with pytest.raises(ScenarioError, match="has no airbase"):
        sc.airbase("Nowhere", ident="NOWHERE")
    with pytest.raises(ScenarioError, match="not a pydcs vehicle type"):
        sc.ground_group("G", country="Russia", at=sc.point(0, 0), units=[planes.Su_27])
    with pytest.raises(ScenarioError, match="exactly 4 points"):
        sc.zone("Quad", points=[sc.point(0, 0), sc.point(1, 0), sc.point(1, 1)])
    with pytest.raises(ScenarioError, match="radius"):
        sc.zone("Circle", at=sc.point(0, 0), radius=0)
    with pytest.raises(ScenarioError, match="air start needs at="):
        sc.player_flight("P", planes.F_16C_50, country="USA", airbase="Senaki-Kolkhi", start=StartType.Air)
    with pytest.raises(ScenarioError, match="ULID"):
        sc.build("not-a-ulid")


def test_durations_and_start_times():
    assert parse_duration("3h") == 10800
    assert parse_duration("1h30m") == 5400
    assert parse_duration("90m") == 5400
    assert parse_duration(45) == 45
    assert parse_duration(None) == 0
    with pytest.raises(ScenarioError):
        parse_duration("soon")
    off = caucasus.Caucasus().utc_offset
    assert parse_start("2026-11-01T06:30:00Z", off).isoformat() == "2026-11-01T06:30:00+00:00"
    # A start without a zone is theatre-local time, as in the mission editor.
    assert parse_start("2026-11-01T10:30:00", off).isoformat() == "2026-11-01T06:30:00+00:00"
    with pytest.raises(ScenarioError, match="required"):
        parse_start(None, off)


def test_roles_are_canonical_in_the_lock(sc):
    sc.ground_group("G", ident="G", country="Russia", at=sc.point(0, 0), units=[SA], roles=["sam", "north", "sam"])
    [g] = sc.lock()["symbols"]["groups"]
    assert g == {"ident": "G", "kind": "ground", "roles": ["north", "sam"]}


def test_geometry_does_not_change_the_hash():
    a, b = scenario(), scenario()
    a.zone("Z", ident="Z", at=a.point(0, 0), radius=10)
    b.zone("Z", ident="Z", at=b.point(5000, 5000), radius=99)
    assert a.schema_sha256() == b.schema_sha256()
    assert a.lock()["geometry"] != b.lock()["geometry"]
    c = scenario(schema=2)
    c.zone("Z", ident="Z", at=c.point(0, 0), radius=10)
    assert c.schema_sha256() != a.schema_sha256()


def test_a_small_scenario_builds(sc):
    sc.airbase("Kutaisi", ident="KUTAISI", coalition=Coalition.Blue)
    sc.ground_group("Armor", ident="ARMOR", country="Russia", at=sc.latlon(42.2, 42.6),
                    units=[vehicles.Armor.T_72B] * 3, formation="line", heading=90)
    sc.air_template("Red Frogfoot", planes.Su_25T, ident="RED_FROGFOOT", country="Russia",
                    airbase="Maykop-Khanskaya")
    sc.flag("ARMOR_DEAD", initial=True)

    @sc.pydcs
    def _(mission):
        mission.weather.clouds_density = 3

    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        b = sc.build()
    m = b.manifest
    assert [a.coalition for a in m.airbases] == [Coalition.Blue.proto]
    assert [u.name for u in m.groups[0].units] == ["Armor-1", "Armor-2", "Armor-3"]
    assert [(t.ident, t.units[0].type_name) for t in m.templates] == [("RED_FROGFOOT", "Su-25T")]
    assert len(m.groups) == 1  # templates stay out of the .miz
    assert [(f.ident, f.initial) for f in m.flags] == [("ARMOR_DEAD", 1)]


def test_offline_build_id_follows_the_mission_content():
    def make(density: int) -> str:
        s = scenario()
        s.ground_group("G", country="Russia", at=s.point(0, 0), units=[SA])

        @s.pydcs
        def _(mission):
            mission.weather.clouds_density = density

        return s.build().build_id

    assert make(2) == make(2)
    assert make(2) != make(5)  # a hook-only change is a different build
