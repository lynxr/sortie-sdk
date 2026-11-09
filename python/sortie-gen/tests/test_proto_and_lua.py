"""The committed protobuf stubs are current, and the bootstrap/lobby Lua behaves in a Lua 5.1 VM."""

from __future__ import annotations

import importlib.util
import sys

import pytest

from sortie_gen import lobby, miz as mizmod

from conftest import PACKAGE, REPO


def _gen_proto():
    spec = importlib.util.spec_from_file_location("gen_proto", PACKAGE / "scripts" / "gen_proto.py")
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules["gen_proto"] = module
    spec.loader.exec_module(module)
    return module


def test_protobuf_stubs_are_up_to_date(tmp_path):
    gen = _gen_proto()
    fresh = tmp_path / "_proto"
    gen.generate(fresh, REPO / "proto")
    stale = gen.diff(fresh, PACKAGE / "src" / "sortie_gen" / "_proto")
    assert not stale, f"run `uv run python scripts/gen_proto.py`; stale: {stale}"


lupa = pytest.importorskip("lupa.lua51")


def run_lua(script: str, *, sortie: bool) -> dict:
    """Runs a DoScript text with stubbed DCS globals and records what it called."""
    lua = lupa.LuaRuntime(unpack_returned_tuples=True)
    lua.execute(
        """
        calls = {}
        trigger = { action = { outText = function(text, seconds)
            calls[#calls + 1] = { fn = "outText", text = text, seconds = seconds } end } }
        """
    )
    if sortie:
        lua.execute(
            """
            SORTIE = {
              attach = function(t) calls[#calls + 1] = { fn = "attach", scenario = t.scenario,
                build_id = t.build_id, digest = t.digest, schema = t.schema } end,
              version = function() return "0.1.0" end,
            }
            """
        )
    lua.execute(script)
    calls = lua.eval("calls")
    return [dict(calls[i].items()) for i in range(1, len(calls) + 1)]


def test_bootstrap_attaches_when_the_bridge_is_loaded(op_north_build):
    calls = run_lua(op_north_build.bootstrap, sortie=True)
    assert calls == [
        {
            "fn": "attach",
            "scenario": "op_north",
            "build_id": op_north_build.build_id,
            "digest": op_north_build.digest,
            "schema": 3,
        }
    ]


def test_bootstrap_says_what_to_do_without_the_bridge(op_north_build):
    calls = run_lua(op_north_build.bootstrap, sortie=False)
    assert calls == [{"fn": "outText", "text": mizmod.NOT_INSTALLED_TEXT, "seconds": 30}]


def test_bootstrap_refuses_values_that_need_escaping():
    from sortie_gen.errors import ScenarioError

    with pytest.raises(ScenarioError):
        mizmod.bootstrap_script("op_north", '01K8"); os.exit()--', "ab", 3)


def test_lobby_script_runs_both_ways():
    with_bridge = run_lua(lobby.LOBBY_SCRIPT, sortie=True)
    assert len(with_bridge) == 1 and "bridge 0.1.0 is ready" in with_bridge[0]["text"]
    without = run_lua(lobby.LOBBY_SCRIPT, sortie=False)
    assert without == [{"fn": "outText", "text": mizmod.NOT_INSTALLED_TEXT, "seconds": 30}]
