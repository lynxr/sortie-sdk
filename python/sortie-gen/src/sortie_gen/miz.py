"""The ``.miz`` side: the bootstrap trigger, the scripts that are not Sortie's (the build's
``do_scripts`` guard, ``sortie-gen adopt``) and a byte-for-byte reproducible zip.

pydcs writes the mission (``Mission.save``); the zip container is then rewritten with fixed
timestamps and attributes, so that building the same scenario twice on one machine gives the
same ``.miz`` bytes. (The compressed bytes depend on the zlib build, so two machines may still
differ in the ``.miz`` bytes; its entries, ``manifest.pb``, ``manifest.json`` and the lock are
meant to match, within the rounding limits described in ``_pydcs``.)
"""

from __future__ import annotations

import hashlib
import io
import re
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict

from . import _pydcs  # noqa: F401  (configures pydcs before dcs is used)
from .errors import ScenarioError

import dcs  # noqa: E402
import dcs.lua  # noqa: E402
from dcs.action import Action, DoScript  # noqa: E402
from dcs.triggers import TriggerStart  # noqa: E402

BOOTSTRAP_COMMENT = "sortie bootstrap"
NOT_INSTALLED_TEXT = "Sortie bridge is not installed: run sortie doctor"
_FIXED_DATE = (1980, 1, 1, 0, 0, 0)

#: Where the docs of the Sortie SDK state the rule a mission's own scripts break (the page of
#: the generator, relative to the SDK repository; its site serves it as ``.html``).
SCRIPTS_RULE_DOCS = "docs/generator.md#логика-миссии-и-do-script"

#: The rule, as the build's warning or error and ``sortie-gen adopt`` say it.
SCRIPTS_RULE = (
    "The .miz holds the map side only (groups, units, statics, zones, weather, briefing, the "
    "warehouses' starting stock) and Sortie's bootstrap; the mission's game logic (what happens "
    "when, spawns, tasks, scoring, menus, marks, stock changes) belongs in the Rust logic: a "
    "typed command or query, a catalog call (ctx.dcs()), a raw DCS task table (Task::raw), or "
    "ask the Sortie team for what is missing. A DO SCRIPT is for a third-party script the "
    f"mission must keep, and Sortie does not check it. See {SCRIPTS_RULE_DOCS} in the Sortie SDK."
)

#: The Mission Editor's names of pydcs's trigger types.
_TRIGGER_KINDS = {
    "triggerStart": "MISSION START",
    "triggerOnce": "ONCE",
    "triggerContinious": "CONTINUOUS",
    "triggerFront": "SWITCHED CONDITION",
}

#: How much of a script's code a report shows.
_SHOWN_CODE = 60

_SAFE = re.compile(r"[A-Za-z0-9_]+")


def bootstrap_script(scenario: str, build_id: str, digest: str, schema: int) -> str:
    """The one-line bootstrap DoScript (the bridge's Lua layer defines ``SORTIE``)."""
    for what, value in (("scenario", scenario), ("build_id", build_id), ("digest", digest)):
        if not _SAFE.fullmatch(value):
            raise ScenarioError(f"bootstrap {what} {value!r} contains characters that need Lua escaping")
    return (
        f'if SORTIE then SORTIE.attach({{scenario="{scenario}",build_id="{build_id}",'
        f'digest="{digest}",schema={int(schema)}}}) '
        f'else trigger.action.outText("{NOT_INSTALLED_TEXT}", 30) end'
    )


class InlineDoScript(Action):
    """A DO SCRIPT action with the code in the action itself, as the Mission Editor writes it.

    pydcs's ``DoScript`` keeps the code in the dictionary (``getValueDictByKey("DictKey_...")``
    plus ``KeyDict_text``). The Mission Editor reads a DO SCRIPT's ``text`` as code, not as a
    dictionary key: a mission re-saved in the editor, or flown from it, then runs the key name as
    Lua (``[string "DictKey_Translation_5"]:1: '=' expected near '<eof>'``). Missions the editor
    saves keep the code inline: ``trig.actions`` holds ``a_do_script("<code>");`` and the
    trigger rule ``{predicate = "a_do_script", text = "<code>"}``, with no dictionary entry.

    pydcs does not keep this form when it loads and saves a mission (its ``DoScript`` writes the
    dictionary form again), so a built ``.miz`` must not be post-processed with pydcs; a
    ``@sc.pydcs`` hook that adds its own DO SCRIPT should use this class. Such a script is for a
    third-party script the mission keeps, never for the mission's logic (``SCRIPTS_RULE``); the
    build warns about it unless the scenario sets ``do_scripts="allow"``.
    """

    predicate = DoScript.predicate

    def __init__(self, code: str) -> None:
        super().__init__(InlineDoScript.predicate)
        self.code = code
        self.params.append(code)

    def dict(self) -> Dict[str, Any]:
        d = super().dict()
        d["text"] = self.code
        return d


def add_start_script(mission: dcs.Mission, script: str, comment: str = BOOTSTRAP_COMMENT) -> TriggerStart:
    """One MISSION START trigger with one DO SCRIPT action (code inline, see InlineDoScript)."""
    trig = TriggerStart(comment=comment)
    trig.add_action(InlineDoScript(script))
    mission.triggerrules.triggers.append(trig)
    return trig


#: Set by ``load`` on each loaded ``DoScript``: whether its trigger rule names a dictionary key
#: (``KeyDict_text``, pydcs's form) rather than holding the code (the editor's form, and ours).
DICTIONARY_FORM = "sortie_dictionary_form"


def do_script_code(action: Any) -> str | None:
    """The code DCS runs for a DO SCRIPT action; None for other actions.

    Built here: ``InlineDoScript``. Loaded with ``load``: a pydcs ``DoScript`` whose rule either
    names a dictionary key (``KeyDict_text``; ``trig.actions`` reads it with ``getValueDictByKey``)
    or holds the code itself. Without ``KeyDict_text`` the text is the code even when it looks
    like a key: a Sortie build re-saved by the editor before this fix holds the text
    ``"DictKey_Translation_5"``, which DCS runs as Lua (and fails), although the dictionary still
    has the old bootstrap under that key. A ``DoScript`` made in memory is the dictionary form.
    """
    if isinstance(action, InlineDoScript):
        return action.code
    if isinstance(action, DoScript):
        text = action.text
        if getattr(action, DICTIONARY_FORM, True) is False:
            return text.id
        translation = getattr(text, "translation", None)
        strings = translation.strings.get("DEFAULT", {}) if translation is not None else {}
        return strings.get(text.id, text.id)
    return None


def start_scripts(mission: dcs.Mission) -> list[tuple[str, list[str]]]:
    """``(comment, [DO SCRIPT code])`` of every MISSION START trigger, in order."""
    out = []
    for trig in mission.triggerrules.triggers:
        if isinstance(trig, TriggerStart):
            codes = [do_script_code(a) for a in trig.actions]
            out.append((trig.comment, [c for c in codes if c is not None]))
    return out


@dataclass(frozen=True)
class MissionScript:
    """Lua of the mission that is not Sortie's own: a DO SCRIPT or DO SCRIPT FILE action or a
    LUA PREDICATE condition of a trigger, or the mission's initialization script or file."""

    #: 1-based position of the trigger among the mission's triggers; 0 for the mission's
    #: initialization script, which belongs to no trigger.
    trigger: int
    #: The trigger's comment (its name in the Mission Editor); empty for the init script.
    comment: str
    #: The trigger's type as the Mission Editor names it: ``MISSION START``, ``ONCE``, ...;
    #: ``INITIALIZATION`` for the init script.
    kind: str
    #: ``DO SCRIPT``, ``DO SCRIPT FILE``, ``LUA PREDICATE`` (a condition), ``INITIALIZATION
    #: SCRIPT`` or ``INITIALIZATION SCRIPT FILE``.
    action: str
    #: The start of the code or the file name.
    what: str

    def describe(self) -> str:
        if self.trigger == 0:
            return f"mission init: {self.action} {self.what}"
        return f"trigger {self.trigger} {self.comment!r} ({self.kind}): {self.action} {self.what}"


def _shown(code: str) -> str:
    lines = [line.strip() for line in code.strip().splitlines() if line.strip()]
    first = lines[0] if lines else ""
    if len(first) > _SHOWN_CODE or len(lines) > 1:
        return first[:_SHOWN_CODE].rstrip() + "..."
    return first


def _script_file(mission: dcs.Mission, key: Any) -> str:
    """The file name of a mission resource (a ``ResourceKey`` or its key)."""
    key = str(getattr(key, "key", key) or "")
    for files in mission.map_resource.files.values():
        if key in files:
            return Path(files[key]).name
    return key or "(no file)"


def _text(mission: dcs.Mission, text: Any) -> str:
    """A text that may name a dictionary entry (pydcs's ``String`` or a key) as the text DCS
    reads: the entry's text when the dictionary has one, else the text itself."""
    key = str(getattr(text, "id", text) or "")
    translation = getattr(text, "translation", None) or mission.translation
    return translation.strings.get("DEFAULT", {}).get(key, key)


def _action_code(mission: dcs.Mission, action: Any) -> str:
    """The code of an action whose predicate is ``a_do_script``, of any class."""
    code = do_script_code(action)
    return code if code is not None else _text(mission, action.dict().get("text", ""))


def mission_scripts(mission: dcs.Mission, *, skip_bootstrap: bool = False) -> list[MissionScript]:
    """All Lua of the mission: its initialization script file and script (DCS runs both when
    the mission starts), then, in trigger order, every LUA PREDICATE condition (DCS runs it each
    time it checks the trigger) and every DO SCRIPT and DO SCRIPT FILE action of a trigger. An
    action or condition is known by its predicate (``a_do_script``, ``a_do_script_file``,
    ``c_predicate``), whatever its class. Lua in a group's waypoint tasks (``WrappedAction``
    ``Script``) is not looked for.

    Sortie's own triggers hold no other script: the flags trigger sets flags only, and the
    build adds the bootstrap after it looked (so a hook's script that imitates it is listed);
    Sortie sets no init script. ``skip_bootstrap`` leaves out a DO SCRIPT that is a Sortie
    bootstrap, for a mission a Sortie build made (``sortie-gen adopt``).
    """
    out = []
    init_file = getattr(mission, "init_script_file", None)
    if init_file:
        what = _script_file(mission, init_file)
        out.append(MissionScript(0, "", "INITIALIZATION", "INITIALIZATION SCRIPT FILE", what))
    init_code = getattr(mission, "init_script", None)
    if init_code and init_code.strip():
        out.append(MissionScript(0, "", "INITIALIZATION", "INITIALIZATION SCRIPT", _shown(init_code)))
    for n, trig in enumerate(mission.triggerrules.triggers, start=1):
        kind = _TRIGGER_KINDS.get(getattr(trig, "predicate", ""), type(trig).__name__)
        for c in trig.rules:
            if getattr(c, "predicate", None) == "c_predicate":
                text = getattr(c, "text", None) or c.dict().get("text", "")
                out.append(MissionScript(n, trig.comment, kind, "LUA PREDICATE", _shown(_text(mission, text))))
        for a in trig.actions:
            predicate = getattr(a, "predicate", None)
            if predicate == "a_do_script_file":
                key = getattr(a, "file_res_key", None) or a.dict().get("file", "")
                out.append(MissionScript(n, trig.comment, kind, "DO SCRIPT FILE", _script_file(mission, key)))
            elif predicate == "a_do_script":
                code = _action_code(mission, a)
                if skip_bootstrap and parse_bootstrap(code) is not None:
                    continue
                out.append(MissionScript(n, trig.comment, kind, "DO SCRIPT", _shown(code)))
    return out


def scripts_report(scripts: list[MissionScript], *, deny: bool = False) -> str:
    """The build's warning (or, with ``deny``, its error) about ``scripts``, with the rule."""
    count = f"{len(scripts)} script{'s' if len(scripts) != 1 else ''}"
    head = f"the .miz runs {count} that Sortie did not put there"
    lines = [head + ":"] + [f"  {s.describe()}" for s in scripts] + [SCRIPTS_RULE]
    if deny:
        lines.append('This scenario sets do_scripts="deny": move the logic to Rust, or set "allow" for a third-party script.')
    else:
        lines.append('Scenario(do_scripts="allow") silences this warning; "deny" fails the build.')
    return "\n".join(lines)


def find_bootstrap(mission: dcs.Mission) -> str | None:
    """The bootstrap script of a mission, if it has one."""
    for _comment, texts in start_scripts(mission):
        for t in texts:
            if "SORTIE.attach(" in t:
                return t
    return None


_ATTACH_RE = re.compile(
    r'SORTIE\.attach\(\{scenario="(?P<scenario>[^"]*)",build_id="(?P<build_id>[^"]*)",'
    r'digest="(?P<digest>[^"]*)",schema=(?P<schema>\d+)\}\)'
)


def parse_bootstrap(script: str) -> dict[str, Any] | None:
    m = _ATTACH_RE.search(script)
    if not m:
        return None
    d: dict[str, Any] = m.groupdict()
    d["schema"] = int(d["schema"])
    return d


def normalize_zip(raw: bytes) -> bytes:
    """Rewrites a zip with fixed timestamps/attributes, keeping entry order and contents."""
    out = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(raw)) as src, zipfile.ZipFile(out, "w") as dst:
        for info in src.infolist():
            data = src.read(info)
            zi = zipfile.ZipInfo(info.filename, date_time=_FIXED_DATE)
            zi.compress_type = zipfile.ZIP_DEFLATED
            zi.create_system = 0
            zi.external_attr = 0
            dst.writestr(zi, data)
    return out.getvalue()


def content_sha256(raw: bytes) -> bytes:
    """sha256 over the entry names and uncompressed contents of a ``.miz`` (not its zip bytes,
    which depend on the zlib build)."""
    h = hashlib.sha256()
    with zipfile.ZipFile(io.BytesIO(raw)) as z:
        for info in z.infolist():
            data = z.read(info)
            name = info.filename.encode("utf-8")
            h.update(len(name).to_bytes(4, "little") + name + len(data).to_bytes(8, "little") + data)
    return h.digest()


def mission_bytes(mission: dcs.Mission) -> bytes:
    """The ``.miz`` bytes pydcs would write, in a reproducible container."""
    with tempfile.TemporaryDirectory(prefix="sortie-gen-miz-") as tmp:
        path = Path(tmp) / "mission.miz"
        mission.save(str(path))
        raw = path.read_bytes()
    return normalize_zip(raw)


def load(path: str | Path) -> dcs.Mission:
    """Opens a ``.miz`` with pydcs (the terrain is taken from the file).

    pydcs drops ``KeyDict_text`` when it reads a DO SCRIPT, and that key is what tells the two
    storage forms apart, so each loaded ``DoScript`` gets ``DICTIONARY_FORM`` from the raw rule.
    """
    m = dcs.Mission()
    m.load_file(str(path))
    with zipfile.ZipFile(path) as z:
        raw = dcs.lua.loads(z.read("mission").decode("utf-8")).get("mission", {})
    rules = raw.get("trigrules") or {}
    # The same walk as pydcs's TriggerRules.load_from_dict / TriggerRule.create_from_dict.
    for trig, key in zip(m.triggerrules.triggers, sorted(rules)):
        raw_actions = rules[key].get("actions") or {}
        for action, a in zip(trig.actions, raw_actions):
            if isinstance(action, DoScript):
                setattr(action, DICTIONARY_FORM, "KeyDict_text" in raw_actions[a])
    return m
