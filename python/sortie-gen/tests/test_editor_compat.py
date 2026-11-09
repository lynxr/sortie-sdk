"""Generated ``.miz`` files survive the DCS Mission Editor.

The editor reads a DO SCRIPT action's ``text`` as code, never as a dictionary key: missions it
saves keep the code inline (``a_do_script("<code>");`` in ``trig.actions`` and ``{predicate =
"a_do_script", text = "<code>"}`` in the trigger rule, no ``KeyDict_text``). pydcs's own
``DoScript`` stores the code in the dictionary instead, so a mission opened in the editor and
re-saved, or flown from it, ran the key name as Lua (live run on DCS 2.9.31: ``[string
"DictKey_Translation_5"]:1: '=' expected near '<eof>'``, no ``SORTIE.attach``).
"""

from __future__ import annotations

import io
import re
import warnings
import zipfile

import dcs
import pytest
from dcs.action import DoScript
from dcs.triggers import TriggerStart

from sortie_gen import adopt as adoptmod, lobby, miz as mizmod


def _lua(miz: bytes):
    """A Lua 5.1 state with the ``mission`` and ``dictionary`` tables of a ``.miz``."""
    lupa = pytest.importorskip("lupa.lua51")
    with zipfile.ZipFile(io.BytesIO(miz)) as z:
        mission = z.read("mission").decode("utf-8")
        dictionary = z.read("l10n/DEFAULT/dictionary").decode("utf-8")
    lua = lupa.LuaRuntime(unpack_returned_tuples=True)
    lua.execute(mission)
    lua.execute(dictionary)
    return lua, mission


def _rules(lua) -> list[tuple[str, list[dict]]]:
    """``(trig.actions[i], [rule i's actions])`` for every trigger rule, in order."""
    out = []
    rules = lua.eval("mission.trigrules")
    actions = lua.eval("mission.trig.actions")
    for i in range(1, len(rules) + 1):
        acts = rules[i].actions
        out.append((actions[i], [dict(acts[k].items()) for k in range(1, len(acts) + 1)]))
    return out


def _runs(lua, trig_action: str) -> list[str]:
    """The DO SCRIPT code a ``trig.actions`` string passes to DCS when the trigger fires."""
    lua.execute(
        """
        captured = {}
        a_do_script = function(code) captured[#captured + 1] = code end
        a_set_flag_value = function() end
        getValueDictByKey = function(key) return dictionary[key] end
        """
    )
    run = lua.eval("function(src) assert(loadstring(src))() end")
    run(trig_action)
    captured = lua.eval("captured")
    return [captured[i] for i in range(1, len(captured) + 1)]


def _builds(op_north_build):
    return [
        ("op_north", op_north_build.miz, op_north_build.bootstrap),
        ("lobby", lobby.build_lobby(), lobby.LOBBY_SCRIPT),
    ]


def test_do_script_code_is_inline_like_the_mission_editor_writes_it(op_north_build):
    for name, miz, code in _builds(op_north_build):
        lua, _ = _lua(miz)
        scripts = []
        for trig_action, actions in _rules(lua):
            for a in (a for a in actions if a["predicate"] == "a_do_script"):
                assert "KeyDict_text" not in a, (name, a)
                scripts.append(a["text"])
            if any(a["predicate"] == "a_do_script" for a in actions):
                assert "getValueDictByKey" not in trig_action, (name, trig_action)
                # What the editor keeps after a re-save (the rule's text, read as code) is what
                # the mission runs now: re-saving changes nothing.
                assert _runs(lua, trig_action) == [
                    a["text"] for a in actions if a["predicate"] == "a_do_script"
                ], name
        assert scripts == [code], name


def test_the_dictionary_holds_every_key_the_mission_names_and_no_code(op_north_build):
    for name, miz, code in _builds(op_north_build):
        lua, mission = _lua(miz)
        dictionary = lua.eval("dictionary")
        keys = {k for k in dictionary.keys()}
        named = set(re.findall(r'"(DictKey_[A-Za-z]+_\d+)"', mission))
        assert named <= keys, (name, named - keys)
        numbers = [int(k.rsplit("_", 1)[1]) for k in keys]
        assert lua.eval("mission.maxDictId") >= max(numbers, default=0), name
        assert all(code not in str(dictionary[k]) for k in keys), name


def test_do_scripts_read_back_in_both_storage_forms(tmp_path):
    """pydcs-style files (code in the dictionary) and editor-style files (code inline)."""
    m = dcs.Mission(dcs.terrain.Caucasus())
    old = TriggerStart(comment="old")
    old.add_action(DoScript(m.string("return 1")))
    m.triggerrules.triggers.append(old)
    mizmod.add_start_script(m, 'trigger.action.outText("x", 1)', comment="new")
    path = tmp_path / "both.miz"
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        path.write_bytes(mizmod.mission_bytes(m))
        loaded = mizmod.load(path)
    assert mizmod.start_scripts(loaded) == [
        ("old", ["return 1"]),
        ("new", ['trigger.action.outText("x", 1)']),
    ]


def test_code_that_needs_escaping_survives_both_places(tmp_path):
    """Newlines, quotes, backslashes, long-bracket closers and non-ASCII text round-trip."""
    code = 'local s = "a \\"quoted\\" \\\\ path"\n-- ]] and ]=] stay text\ntrigger.action.outText(s .. "ё", 5)\n'
    assert "\\\\" in code and '\\"' in code and "\n" in code  # the test really has them
    m = dcs.Mission(dcs.terrain.Caucasus())
    mizmod.add_start_script(m, code, comment="tricky")
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        miz = mizmod.mission_bytes(m)
    lua, _ = _lua(miz)
    [(trig_action, actions)] = _rules(lua)
    assert actions == [{"predicate": "a_do_script", "text": code}]
    assert _runs(lua, trig_action) == [code]
    (tmp_path / "t.miz").write_bytes(miz)
    assert mizmod.start_scripts(mizmod.load(tmp_path / "t.miz")) == [("tricky", [code])]


def _resave_like_the_old_editor_did(miz: bytes, key: str) -> bytes:
    """What the Mission Editor made of a pre-fix build (DCS's own temp mission after a re-save):
    the action runs the key name, the rule keeps ``text = <key>`` without ``KeyDict_text``, and
    the dictionary still holds the old code under that key."""
    with zipfile.ZipFile(io.BytesIO(miz)) as z:
        entries = [(i, z.read(i)) for i in z.infolist()]
    out = io.BytesIO()
    with zipfile.ZipFile(out, "w") as z:
        for info, data in entries:
            if info.filename == "mission":
                text = data.decode("utf-8")
                text, n = re.subn(r'\["KeyDict_text"\]="' + re.escape(key) + r'",\s*', "", text)
                assert n == 1, "the pre-fix rule names its key"
                old = 'getValueDictByKey(\\"' + key + '\\")'
                assert text.count(old) == 1, "the pre-fix action reads the dictionary"
                text = text.replace(old, '\\"' + key + '\\"')
                data = text.encode("utf-8")
            z.writestr(info, data)
    return out.getvalue()


def test_an_old_build_resaved_by_the_editor_has_no_working_bootstrap(op_north_build, tmp_path):
    """DCS runs the key name there (and fails), so readers must not report the stale code."""
    m = dcs.Mission(dcs.terrain.Caucasus())
    trig = TriggerStart(comment=mizmod.BOOTSTRAP_COMMENT)
    trig.add_action(DoScript(m.string(op_north_build.bootstrap)))  # the pre-fix form
    m.triggerrules.triggers.append(trig)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        before = mizmod.mission_bytes(m)
    key = trig.actions[0].text.id
    assert key.startswith("DictKey_")

    # Before the re-save the old form still works (pydcs files keep KeyDict_text).
    (tmp_path / "before.miz").write_bytes(before)
    assert mizmod.find_bootstrap(mizmod.load(tmp_path / "before.miz")) == op_north_build.bootstrap

    after = _resave_like_the_old_editor_did(before, key)
    lua, _ = _lua(after)
    [(trig_action, _actions)] = _rules(lua)
    assert _runs(lua, trig_action) == [key], "DCS runs the key name"
    assert lua.eval("dictionary")[key] == op_north_build.bootstrap, "the stale code is still there"
    path = tmp_path / "after.miz"
    path.write_bytes(after)
    loaded = mizmod.load(path)
    assert mizmod.start_scripts(loaded) == [(mizmod.BOOTSTRAP_COMMENT, [key])]
    assert mizmod.find_bootstrap(loaded) is None
    assert adoptmod.adopt(path).has_bootstrap is False
