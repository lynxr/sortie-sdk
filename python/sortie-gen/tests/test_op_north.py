"""op_north end to end, offline: the example scenario, its .miz, its manifest and its lock."""

from __future__ import annotations

import hashlib
import json
import math
import os
import subprocess
import sys
from datetime import datetime, timedelta

import dcs
import pytest

from sortie_gen import manifest as manifestmod
from sortie_gen import miz as mizmod
from sortie_gen._proto.sortie.manifest.v1 import manifest_pb2 as mpb
from sortie_gen._proto.sortie.v1 import common_pb2
from sortie_gen import lock as lockmod
from sortie_gen import _pydcs

from conftest import FIXTURES, OP_NORTH, OP_NORTH_SCENARIO, OP_NORTH_SCHEMA_SHA256, PACKAGE


@pytest.fixture(scope="module")
def mission(op_north_build, tmp_path_factory):
    path = tmp_path_factory.mktemp("miz") / "op_north.miz"
    path.write_bytes(op_north_build.miz)
    return mizmod.load(path)


def test_schema_hash_matches_the_rust_golden(op_north):
    assert op_north.schema_sha256() == OP_NORTH_SCHEMA_SHA256
    assert op_north.lock()["schema_sha256"] == OP_NORTH_SCHEMA_SHA256


def test_committed_lock_is_what_the_generator_writes(op_north):
    committed = (OP_NORTH / "scenario.lock.json").read_text(encoding="utf-8")
    assert committed == op_north.lock_text()
    doc = lockmod.parse_lock(committed)
    assert doc["schema_sha256"] == OP_NORTH_SCHEMA_SHA256
    assert committed.endswith("}\n") and "\r" not in committed


def test_lock_layout_is_the_contract_layout(op_north):
    lock = op_north.lock()
    expected = json.dumps(lock, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    assert op_north.lock_text() == expected
    hashed = json.dumps(
        {"scenario": lock["scenario"], "schema": lock["schema"], "symbols": lock["symbols"]},
        sort_keys=True,
        ensure_ascii=False,
        separators=(",", ":"),
    ).encode("utf-8")
    assert hashlib.sha256(hashed).hexdigest() == lock["schema_sha256"]
    # Geometry is in the lock for the macros, but it is not hashed.
    assert set(lock["geometry"]["zones"]) == {"OBJECTIVE", "SAM_RING"}


def test_committed_fixtures_are_up_to_date(op_north_build):
    """`python scenario.py build --out logic/fixtures --stem op_north --no-miz` reproduces them."""
    assert (FIXTURES / "op_north.manifest.pb").read_bytes() == op_north_build.manifest_pb
    assert (FIXTURES / "op_north.manifest.json").read_text(encoding="utf-8") == op_north_build.manifest_json


#: pyproj results, Linux vs Windows (pyproj 3.8.0, PROJ 9.8.1), for op_north units as they were
#: before the rounding (the old, unrounded zone centre): the case the rounding exists for.
CROSS_PLATFORM_LATLON = [
    (44.61999999999999, 44.61999999999998),
    (44.619642664863115, 44.61964266486311),
    (40.09949967827678, 40.09949967827677),
    (40.03526520187906, 40.035265201879064),
]


@pytest.mark.parametrize(("linux", "windows"), CROSS_PLATFORM_LATLON)
def test_quantization_absorbs_the_platform_difference(linux, windows):
    assert linux != windows
    assert _pydcs._quantize(linux, _pydcs.LATLON_DIGITS) == _pydcs._quantize(windows, _pydcs.LATLON_DIGITS)


def test_projected_values_are_quantized(op_north, op_north_build):
    """Unit latlon in the manifest, the lock's zone centres and sc.latlon points are rounded."""
    m = op_north_build.manifest
    units = [u for coll in (m.groups, m.templates, m.player_flights) for g in coll for u in g.units]
    assert units
    for u in units:
        for v in (u.latlon.lat, u.latlon.lon):
            assert v == round(v, _pydcs.LATLON_DIGITS), (u.name, v)
    # The objective (and the SAM ring around it) is placed with sc.latlon(...).
    for ident in ("OBJECTIVE", "SAM_RING"):
        center = op_north.lock()["geometry"]["zones"][ident]["circle"]["center"]
        for v in (center["north"], center["east"]):
            assert v == round(v, _pydcs.POINT_DIGITS), (ident, v)
    p = op_north.latlon(44.62, 40.10)
    assert (p.x, p.y) == (round(p.x, _pydcs.POINT_DIGITS), round(p.y, _pydcs.POINT_DIGITS))
    lat, lon = _pydcs.latlon(op_north.theatre, p.x, p.y)
    assert abs(lat - 44.62) < 1e-7 and abs(lon - 40.10) < 1e-7


def test_manifest_decodes_and_names_every_ident(op_north_build):
    m = manifestmod.decode(op_north_build.manifest_pb)
    assert m.scenario == "op_north" and m.schema == 3 and m.build_id == op_north_build.build_id
    assert m.schema_sha256 == OP_NORTH_SCHEMA_SHA256
    assert m.theatre == "Caucasus"
    assert m.time_limit_s == 3 * 3600
    assert m.start_time == "2026-11-01T06:30:00Z"
    assert [(g.ident, g.name) for g in m.groups] == [("SAM_NORTH", "SAM North")]
    assert list(m.groups[0].roles) == ["north", "sam"]
    assert [u.name for u in m.groups[0].units] == [f"SAM North-{k}" for k in range(1, 8)]
    assert [(z.ident, z.name) for z in m.zones] == [("OBJECTIVE", "Objective"), ("SAM_RING", "SAM Ring")]
    assert [(t.ident, t.name) for t in m.templates] == [("RED_CAP", "Red CAP")]
    assert [u.name for u in m.templates[0].units] == ["Red CAP-1", "Red CAP-2"]
    assert [(a.ident, a.name, a.airbase_id) for a in m.airbases] == [("MAYKOP_KHANSKAYA", "Maykop-Khanskaya", 16)]
    assert [(f.ident, f.name) for f in m.flags] == [("SAM_NORTH_DOWN", "SAM_NORTH_DOWN")]
    assert [(x.ident, x.label) for x in m.menus] == [("SMOKE_OBJECTIVE", "Smoke the objective")]
    assert [(p.name, p.size, p.airbase) for p in m.player_flights] == [("Blue 1", 2, "Senaki-Kolkhi")]
    assert m.player_flights[0].start == common_pb2.START_TYPE_COLD
    assert m.max_ids.group_id == 2 and m.max_ids.unit_id == 9
    assert m.tools.generator == "sortie-gen 0.1.0" and m.tools.pydcs == "1.0.0"


def test_manifest_json_and_pb_agree(op_north_build):
    from google.protobuf import json_format

    m = json_format.Parse(op_north_build.manifest_json, mpb.Manifest())
    assert manifestmod.encode(m) == op_north_build.manifest_pb
    assert manifestmod.digest(op_north_build.manifest_pb) == op_north_build.digest
    assert op_north_build.digest == hashlib.sha256(op_north_build.manifest_pb).hexdigest()


def test_template_carries_its_dcs_group_but_is_not_in_the_miz(op_north_build, mission):
    t = op_north_build.manifest.templates[0]
    group = json.loads(t.dcs_group_json)
    assert group["name"] == "Red CAP"
    assert [u["type"] for u in group["units"]] == ["Su-27", "Su-27"]
    # The engine assigns ids, callsigns and parking at spawn time.
    assert "groupId" not in group
    assert all("unitId" not in u and "callsign" not in u for u in group["units"])
    names = {g.name for side in mission.coalition.values() for c in side.countries.values() for g in c.plane_group}
    assert "Red CAP" not in names


def test_miz_has_the_bootstrap_trigger(op_north_build, mission):
    scripts = mizmod.start_scripts(mission)
    assert [c for c, _ in scripts].count(mizmod.BOOTSTRAP_COMMENT) == 1
    text = mizmod.find_bootstrap(mission)
    assert text == op_north_build.bootstrap
    assert text == (
        'if SORTIE then SORTIE.attach({scenario="op_north",'
        f'build_id="{op_north_build.build_id}",digest="{op_north_build.digest}",schema=3}}) '
        'else trigger.action.outText("Sortie bridge is not installed: run sortie doctor", 30) end'
    )
    assert mizmod.parse_bootstrap(text) == {
        "scenario": "op_north",
        "build_id": op_north_build.build_id,
        "digest": op_north_build.digest,
        "schema": 3,
    }


def test_miz_groups_and_coordinates(op_north_build, mission):
    m = op_north_build.manifest
    red = mission.coalition["red"].countries["Russia"]
    [sam] = red.vehicle_group
    assert sam.name == "SAM North" and int(sam.id) == m.groups[0].group_id
    assert [u.name for u in sam.units] == [u.name for u in m.groups[0].units]
    for unit, mu in zip(sam.units, m.groups[0].units, strict=True):
        assert int(unit.id) == mu.unit_id
        assert math.isclose(unit.position.x, mu.position.north, abs_tol=1e-6)
        assert math.isclose(unit.position.y, mu.position.east, abs_tol=1e-6)
    # The command post sits on the objective (44.62 N, 40.10 E).
    lat, lon = sam.units[0].position.latlng().lat, sam.units[0].position.latlng().lng
    assert math.isclose(lat, 44.62, abs_tol=1e-6) and math.isclose(lon, 40.10, abs_tol=1e-6)

    [blue] = mission.coalition["blue"].countries["USA"].plane_group
    assert blue.name == "Blue 1" and [u.name for u in blue.units] == ["Blue 1-1", "Blue 1-2"]
    assert all(u.skill == dcs.unit.Skill.Client for u in blue.units)
    assert blue.points[0].type == "TakeOffParking"

    zones = {z.name: z for z in mission.triggers.zones()}
    assert set(zones) == {"Objective", "SAM Ring"}
    assert zones["SAM Ring"].radius == 60000
    assert math.isclose(zones["Objective"].position.x, m.zones[0].circle.center.north, abs_tol=1e-6)


def test_start_time_is_written_in_theatre_local_time(mission):
    # 06:30Z on the Caucasus map (UTC+4) is 10:30 in the mission editor.
    # pydcs reads the date back without a zone.
    assert mission.start_time.replace(tzinfo=None) == datetime(2026, 11, 1, 10, 30)
    assert mission.terrain.utc_offset.utcoffset(None) == timedelta(hours=4)


def test_build_twice_is_byte_identical(op_north):
    import warnings

    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        a, b = op_north.build(), op_north.build()
    assert a.build_id == b.build_id
    assert a.manifest_pb == b.manifest_pb
    assert a.manifest_json == b.manifest_json
    assert a.miz == b.miz


def test_explicit_build_id_changes_only_the_id_and_digest(op_north):
    import warnings

    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        a = op_north.build("01K8Y2Q0PH0000000000000000")
    assert a.build_id == "01K8Y2Q0PH0000000000000000" == a.manifest.build_id
    assert a.bootstrap.count("01K8Y2Q0PH0000000000000000") == 1


def test_build_is_reproducible_across_processes(op_north_build, tmp_path):
    """Two interpreters with different hash seeds write the same bytes."""
    outs = []
    for seed in ("1", "2"):
        out = tmp_path / seed
        env = {**os.environ, "PYTHONHASHSEED": seed}
        r = subprocess.run(
            [sys.executable, str(OP_NORTH_SCENARIO), "build", "--locked", "--out", str(out), "--stem", "op_north"],
            cwd=PACKAGE,
            env=env,
            capture_output=True,
            text=True,
        )
        assert r.returncode == 0, r.stderr
        outs.append({p.name: p.read_bytes() for p in sorted(out.iterdir())})
    assert outs[0] == outs[1]
    assert outs[0]["op_north.manifest.pb"] == op_north_build.manifest_pb
    assert outs[0]["op_north.miz"] == op_north_build.miz


def test_hermetic_build_warns_about_the_unarmed_template(op_north_build):
    assert any("Red CAP" in w and "without weapons" in w for w in op_north_build.warnings)
