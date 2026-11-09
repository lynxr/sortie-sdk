"""Offline CLI commands: lock, build, lobby, adopt."""

from __future__ import annotations

import json
import shutil
import warnings

import pytest

from sortie_gen import adopt as adoptmod, cli, lobby, miz as mizmod
from sortie_gen.cli import load_scenario

from conftest import OP_NORTH, REPO


@pytest.fixture
def scenario_copy(tmp_path):
    """op_north copied to a temp folder, so lock/build write there and not into the repo."""
    d = tmp_path / "op_north"
    d.mkdir()
    shutil.copy(OP_NORTH / "scenario.py", d / "scenario.py")
    shutil.copy(OP_NORTH / "scenario.lock.json", d / "scenario.lock.json")
    return d


def test_lock_check_passes_on_the_committed_lock(scenario_copy, ui):
    sc = load_scenario(scenario_copy / "scenario.py")
    assert cli.scenario_main(sc, ["lock", "--check"], ui) == 0
    assert "is up to date (hash 066934cb9397)" in ui.stdout


def test_stale_lock_fails_with_locked_and_is_rewritten_otherwise(scenario_copy, ui):
    lock = scenario_copy / "scenario.lock.json"
    lock.write_text(lock.read_text().replace('"SAM_RING"', '"SAM_RINGS"'))
    sc = load_scenario(scenario_copy / "scenario.py")
    assert cli.scenario_main(sc, ["lock", "--check"], ui) == 1
    assert "is stale (the file is invalid: schema_sha256 mismatch" in ui.stderr
    assert "new symbols: zone:SAM_RING; removed symbols: zone:SAM_RINGS" in ui.stderr
    assert cli.scenario_main(sc, ["build", "--locked", "--no-miz"], ui) == 1
    assert cli.scenario_main(sc, ["lock"], ui) == 0
    assert "updated" in ui.stdout
    assert lock.read_text() == (OP_NORTH / "scenario.lock.json").read_text()


def test_build_writes_three_files_atomically(scenario_copy, ui):
    sc = load_scenario(scenario_copy / "scenario.py")
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        assert cli.scenario_main(sc, ["build", "--stem", "op_north"], ui) == 0
    dist = scenario_copy / "dist"
    assert sorted(p.name for p in dist.iterdir()) == ["op_north.manifest.json", "op_north.manifest.pb", "op_north.miz"]
    assert "spawns without weapons" in ui.stderr  # the hermetic-payload warning reaches the user


def test_scenario_file_without_a_scenario(tmp_path):
    f = tmp_path / "empty.py"
    f.write_text("x = 1\n")
    with pytest.raises(cli.GeneratorError, match="defines no Scenario"):
        load_scenario(f)


def test_main_dispatches_scenario_files(scenario_copy, ui):
    assert cli.main(["lock", str(scenario_copy / "scenario.py"), "--check"], ui) == 0
    assert cli.main(["lock", str(scenario_copy / "missing.py")], ui) == 1
    assert "no scenario file" in ui.stderr


def test_lobby_command_and_the_committed_lobby(tmp_path, ui):
    out = tmp_path / "missions" / "sortie-lobby.miz"
    assert cli.main(["lobby", "--out", str(out)], ui) == 0
    committed = REPO / "missions" / "sortie-lobby.miz"
    if committed.exists():  # committed with the engine; the SDK repository does not ship it
        assert out.read_bytes() == committed.read_bytes()
    m = mizmod.load(out)
    assert mizmod.find_bootstrap(m) is None
    [(comment, [text])] = mizmod.start_scripts(m)
    assert comment == lobby.LOBBY_COMMENT and text == lobby.LOBBY_SCRIPT
    assert cli.main(["lobby", "--out", str(out), "--theatre", "atlantis"], ui) == 1
    assert "unknown theatre" in ui.stderr


def test_adopt_lists_groups_and_warns_about_parking(op_north_build, tmp_path, ui):
    path = tmp_path / "op_north.miz"
    path.write_bytes(op_north_build.miz)
    report = adoptmod.adopt(path)
    assert [(g.name, g.category, g.client, g.suggested_ident) for g in report.groups] == [
        ("SAM North", "ground", False, "SAM_NORTH"),
        ("Blue 1", "air", True, "BLUE_1"),
    ]
    assert report.has_bootstrap
    assert adoptmod.PARKING_WARNING in report.warnings
    assert cli.main(["adopt", str(path), "--json"], ui) == 0
    assert json.loads(ui.stdout)["groups"][0]["units"][0] == "SAM North-1"


def test_suggested_idents():
    assert adoptmod.suggest_ident("SAM North") == "SAM_NORTH"
    assert adoptmod.suggest_ident("blue-1 (F-16)") == "BLUE_1_F_16"
    assert adoptmod.suggest_ident("1st Armor") == "G_1ST_ARMOR"
