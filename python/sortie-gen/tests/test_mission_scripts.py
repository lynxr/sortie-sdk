"""The guard against game logic in the ``.miz``: all Lua a hook added (the mission's init script
and init script file, every DO SCRIPT and DO SCRIPT FILE action and LUA PREDICATE condition).

``build`` (and ``register``, which builds) lists them after the hooks ran: a warning by default
(``do_scripts="warn"``), a failed build with ``"deny"`` (the mission template's setting), silence
with ``"allow"``. Sortie's own triggers (the bootstrap, the flags trigger) are never listed.
``sortie-gen adopt`` lists the scripts of an editor mission the same way.
"""

from __future__ import annotations

import json
import warnings
from pathlib import Path

import pytest
from dcs import vehicles
from dcs.action import Action, DoScriptFile, SetFlag
from dcs.condition import Predicate
from dcs.terrain import caucasus
from dcs.triggers import TriggerContinious, TriggerOnce, TriggerStart

from sortie_gen import Scenario, SortieWarning, adopt as adoptmod, cli, miz as mizmod
from sortie_gen.errors import ScenarioError

from conftest import REPO
from test_engine import ENGINE_BUILD_ID, fake_engine  # noqa: F401  (the in-process engine fixture)


def _first_file(*paths: Path) -> Path | None:
    return next((p for p in paths if p.is_file()), None)


# The mission template: sdk/template here, template/ in an exported SDK.
TEMPLATE_SCENARIO = _first_file(REPO / "sdk" / "template" / "scenario.py", REPO / "template" / "scenario.py")
# The page the rule names, relative to the SDK repository (sdk/ here).
RULE_PAGE = _first_file(REPO / "sdk" / mizmod.SCRIPTS_RULE_DOCS.split("#")[0], REPO / mizmod.SCRIPTS_RULE_DOCS.split("#")[0])
needs_template = pytest.mark.skipif(TEMPLATE_SCENARIO is None, reason="no mission template here")


def scenario(
    tmp_path: Path, *, do_scripts: str | None = None, flags: bool = False, name: str = "scripts_demo"
) -> Scenario:
    kw = {} if do_scripts is None else {"do_scripts": do_scripts}
    sc = Scenario(
        name,
        schema=1,
        theatre=caucasus.Caucasus(),
        start="2026-06-01T08:00:00Z",
        source=tmp_path / "scenario.py",
        **kw,
    )
    zone = sc.zone("Target", ident="TARGET", at=sc.latlon(42.27, 42.90), radius=3_000)
    sc.ground_group("Armor", ident="ARMOR", country="Russia", at=zone.center, units=[vehicles.Armor.T_72B])
    if flags:
        sc.flag("STARTED", initial=1)
    return sc


def with_scripts(sc: Scenario, tmp_path: Path) -> Scenario:
    """A hook that adds what a client's agent put in the .miz: logic in two triggers."""
    lua = tmp_path / "convoy_logic.lua"
    lua.write_text("-- moves the convoy\n", encoding="utf-8")

    @sc.pydcs
    def logic(m):
        start = TriggerStart(comment="spawn waves")
        start.add_action(mizmod.InlineDoScript("local waves = {}\n" + "waves[1] = 'armor' " * 10))
        m.triggerrules.triggers.append(start)
        once = TriggerOnce(comment="convoy")
        once.add_action(DoScriptFile(m.map_resource.add_resource_file(lua)))
        m.triggerrules.triggers.append(once)

    return sc


def build(sc: Scenario) -> tuple[object, list[str]]:
    with warnings.catch_warnings(record=True) as records:
        warnings.simplefilter("always", SortieWarning)
        b = sc.build()
    return b, [str(r.message) for r in records if issubclass(r.category, SortieWarning)]


def test_a_build_warns_by_default_and_names_each_trigger(tmp_path):
    sc = with_scripts(scenario(tmp_path), tmp_path)
    assert sc.do_scripts == "warn"
    b, caught = build(sc)
    assert b.miz, "the build still produces the mission"
    [warning] = caught
    assert warning == b.warnings[0]
    lines = warning.splitlines()
    assert lines[0] == "the .miz runs 2 scripts that Sortie did not put there:"
    assert lines[1] == "  trigger 1 'spawn waves' (MISSION START): DO SCRIPT local waves = {}..."
    assert lines[2] == "  trigger 2 'convoy' (ONCE): DO SCRIPT FILE convoy_logic.lua"
    assert "belongs in the Rust logic" in warning
    assert mizmod.SCRIPTS_RULE_DOCS in warning
    assert 'do_scripts="allow"' in warning


def test_deny_fails_the_build_with_the_same_list(tmp_path):
    sc = with_scripts(scenario(tmp_path, do_scripts="deny"), tmp_path)
    with pytest.raises(ScenarioError) as e:
        sc.build()
    text = str(e.value)
    assert "trigger 1 'spawn waves' (MISSION START): DO SCRIPT" in text
    assert "trigger 2 'convoy' (ONCE): DO SCRIPT FILE convoy_logic.lua" in text
    assert 'sets do_scripts="deny"' in text


def test_allow_is_silent(tmp_path):
    sc = with_scripts(scenario(tmp_path, do_scripts="allow"), tmp_path)
    b, caught = build(sc)
    assert caught == [] and b.warnings == []


def test_a_long_line_is_cut(tmp_path):
    sc = scenario(tmp_path)

    @sc.pydcs
    def one(m):
        trig = TriggerStart(comment="long")
        trig.add_action(mizmod.InlineDoScript("x" * 100))
        m.triggerrules.triggers.append(trig)

    _, [warning] = build(sc)
    assert f"trigger 1 'long' (MISSION START): DO SCRIPT {'x' * 60}...\n" in warning


def test_sorties_own_triggers_are_not_listed(tmp_path):
    for policy in ("warn", "deny"):
        sc = scenario(tmp_path, do_scripts=policy, flags=True)
        b, caught = build(sc)
        assert caught == [], policy
        m = mizmod.load(Path(b.write(tmp_path / policy)["miz"]))
        comments = [t.comment for t in m.triggerrules.triggers]
        assert comments == ["sortie flags", mizmod.BOOTSTRAP_COMMENT]
        assert len(mizmod.mission_scripts(m)) == 1  # the bootstrap, seen without the build's skip
        assert mizmod.mission_scripts(m, skip_bootstrap=True) == []


def test_the_policy_is_checked(tmp_path):
    with pytest.raises(ScenarioError, match=r"do_scripts must be one of 'warn', 'deny', 'allow', not 'deny '"):
        scenario(tmp_path, do_scripts="deny ")
    sc = scenario(tmp_path)
    sc.do_scripts = "allow"
    assert sc.do_scripts == "allow"
    with pytest.raises(ScenarioError, match="do_scripts must be one of"):
        sc.do_scripts = True  # type: ignore[assignment]


def test_the_cli_build_prints_the_warning_or_fails(tmp_path, ui):
    common = ["--out", str(tmp_path / "dist"), "--lock", str(tmp_path / "scenario.lock.json")]
    warn = with_scripts(scenario(tmp_path), tmp_path)
    assert cli.scenario_main(warn, ["build", *common], ui) == 0, ui.stderr
    assert "warning: the .miz runs 2 scripts that Sortie did not put there:" in ui.stderr
    deny = with_scripts(scenario(tmp_path, do_scripts="deny"), tmp_path)
    assert cli.scenario_main(deny, ["build", *common], ui) == 1
    assert "error: the .miz runs 2 scripts that Sortie did not put there:" in ui.stderr
    assert "trigger 2 'convoy' (ONCE): DO SCRIPT FILE convoy_logic.lua" in ui.stderr


def test_register_builds_with_the_same_guard(fake_engine, tmp_path, ui):
    common = ["--engine", fake_engine.address, "--no-validate", "--lock", str(tmp_path / "scenario.lock.json")]
    # The fake engine knows one scenario, op_north.
    deny = with_scripts(scenario(tmp_path, do_scripts="deny", name="op_north"), tmp_path)
    assert cli.scenario_main(deny, ["register", *common], ui) == 1
    assert "error: the .miz runs 2 scripts that Sortie did not put there:" in ui.stderr
    assert fake_engine.registered == {}
    warn = with_scripts(scenario(tmp_path, name="op_north"), tmp_path)
    assert cli.scenario_main(warn, ["register", *common], ui) == 0, ui.stderr
    assert "warning: the .miz runs 2 scripts that Sortie did not put there:" in ui.stderr
    assert ENGINE_BUILD_ID in fake_engine.registered


@needs_template
def test_the_template_denies_and_still_builds():
    [sc] = [v for v in _run(TEMPLATE_SCENARIO).values() if isinstance(v, Scenario)]
    assert sc.do_scripts == "deny"
    b, caught = build(sc)
    assert caught == [] and b.miz


@needs_template
def test_the_template_refuses_a_hook_script(tmp_path):
    [sc] = [v for v in _run(TEMPLATE_SCENARIO).values() if isinstance(v, Scenario)]
    with_scripts(sc, tmp_path)
    with pytest.raises(ScenarioError, match="trigger 1 'spawn waves'"):
        sc.build()


def test_adopt_lists_the_scripts_but_not_the_bootstrap(tmp_path, ui):
    sc = with_scripts(scenario(tmp_path, do_scripts="allow"), tmp_path)
    path = Path(sc.build().write(tmp_path / "dist")["miz"])
    report = adoptmod.adopt(path)
    assert report.has_bootstrap
    assert [s["trigger"] for s in report.scripts] == [1, 2]
    assert report.scripts[1] == {
        "trigger": 2,
        "comment": "convoy",
        "kind": "ONCE",
        "action": "DO SCRIPT FILE",
        "what": "convoy_logic.lua",
    }
    assert mizmod.SCRIPTS_RULE in report.warnings
    text = adoptmod.render(report)
    assert "  scripts that are not Sortie's (move their logic to Rust):" in text
    assert "    trigger 1 'spawn waves' (MISSION START): DO SCRIPT local waves = {}..." in text
    assert "    trigger 2 'convoy' (ONCE): DO SCRIPT FILE convoy_logic.lua" in text
    assert cli.main(["adopt", str(path), "--json"], ui) == 0
    assert len(json.loads(ui.stdout)["scripts"]) == 2


def test_adopt_of_a_clean_build_lists_no_script(tmp_path):
    path = Path(scenario(tmp_path, flags=True).build().write(tmp_path / "dist")["miz"])
    report = adoptmod.adopt(path)
    assert report.scripts == []
    assert mizmod.SCRIPTS_RULE not in report.warnings
    assert "scripts that are not Sortie's" not in adoptmod.render(report)


def with_lua_elsewhere(sc: Scenario, tmp_path: Path) -> Scenario:
    """A hook that puts game logic where no trigger action holds it: the mission's init script
    and init script file (DCS runs both when the mission starts), a LUA PREDICATE condition (DCS
    runs it each time it checks the trigger) and an action of its own class that is a DO SCRIPT
    by its predicate only."""
    lua = tmp_path / "waves.lua"
    lua.write_text("-- spawns the waves\n", encoding="utf-8")

    class OwnDoScript(Action):
        def __init__(self, code: str) -> None:
            super().__init__("a_do_script")
            self.code = code
            self.params.append(code)

        def dict(self):
            d = super().dict()
            d["text"] = self.code
            return d

    @sc.pydcs
    def logic(m):
        m.init_script = "timer.scheduleFunction(spawnWaves, nil, 60)"
        m.init_script_file = m.map_resource.add_resource_file(lua).key
        every = TriggerContinious(comment="every second")
        every.add_condition(Predicate(m.string("return checkWaves()")))
        every.add_action(SetFlag(1))
        m.triggerrules.triggers.append(every)
        own = TriggerOnce(comment="own class")
        own.add_action(OwnDoScript("moveConvoy()"))
        m.triggerrules.triggers.append(own)

    return sc


LUA_ELSEWHERE = [
    "  mission init: INITIALIZATION SCRIPT FILE waves.lua",
    "  mission init: INITIALIZATION SCRIPT timer.scheduleFunction(spawnWaves, nil, 60)",
    "  trigger 1 'every second' (CONTINUOUS): LUA PREDICATE return checkWaves()",
    "  trigger 2 'own class' (ONCE): DO SCRIPT moveConvoy()",
]


def test_the_init_script_a_lua_predicate_and_an_own_action_class_are_listed(tmp_path):
    _, [warning] = build(with_lua_elsewhere(scenario(tmp_path), tmp_path))
    lines = warning.splitlines()
    assert lines[0] == "the .miz runs 4 scripts that Sortie did not put there:"
    assert lines[1:5] == LUA_ELSEWHERE


def test_deny_fails_on_lua_outside_the_trigger_actions(tmp_path):
    sc = with_lua_elsewhere(scenario(tmp_path, do_scripts="deny"), tmp_path)
    with pytest.raises(ScenarioError) as e:
        sc.build()
    for line in LUA_ELSEWHERE:
        assert line in str(e.value)


def test_an_empty_init_script_is_not_listed(tmp_path):
    sc = scenario(tmp_path, do_scripts="deny")

    @sc.pydcs
    def blank(m):
        m.init_script = "  \n"

    b, caught = build(sc)
    assert caught == [] and b.miz


def test_adopt_lists_the_init_script_and_a_lua_predicate(tmp_path):
    sc = with_lua_elsewhere(scenario(tmp_path, do_scripts="allow"), tmp_path)
    path = Path(sc.build().write(tmp_path / "dist")["miz"])
    report = adoptmod.adopt(path)
    assert report.scripts[0] == {
        "trigger": 0,
        "comment": "",
        "kind": "INITIALIZATION",
        "action": "INITIALIZATION SCRIPT FILE",
        "what": "waves.lua",
    }
    text = adoptmod.render(report)
    for line in LUA_ELSEWHERE:
        assert f"  {line}" in text
    assert mizmod.SCRIPTS_RULE in report.warnings


@pytest.mark.skipif(RULE_PAGE is None, reason="no docs here")
def test_the_rule_names_a_section_the_docs_have():
    page, anchor = mizmod.SCRIPTS_RULE_DOCS.split("#")
    assert page == "docs/generator.md"
    text = RULE_PAGE.read_text(encoding="utf-8")
    headings = [line[3:].strip() for line in text.splitlines() if line.startswith("## ")]
    # GitHub's anchor: lower case, spaces as '-', other punctuation dropped.
    anchors = {"".join(c for c in h.lower().replace(" ", "-") if c.isalnum() or c in "-_") for h in headings}
    assert anchor in anchors, sorted(anchors)


def _run(path: Path) -> dict:
    import runpy

    return runpy.run_path(str(path), run_name="__sortie_test__")
