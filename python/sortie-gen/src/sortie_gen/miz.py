"""The ``.miz`` side: the bootstrap trigger and a byte-for-byte reproducible zip.

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
    ``@sc.pydcs`` hook that adds its own DO SCRIPT should use this class.
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
