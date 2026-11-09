"""``scenario.lock.json``: symbols, kinds, roles, the schema hash and zone geometry.

The rules are shared with the Rust side (``sortie-manifest``) byte for byte:

* layout: ``json.dumps(lock, indent=2, sort_keys=True, ensure_ascii=False) + "\\n"``;
* schema hash: ``sha256(json.dumps({"scenario", "schema", "symbols"}, sort_keys=True,
  ensure_ascii=False, separators=(",", ":")).encode())`` in lowercase hex. Geometry, the
  generator string and the format number are not hashed: moving a zone needs no rebuild of
  the logic, renaming a symbol does.
"""

from __future__ import annotations

import hashlib
import json
import re
from collections.abc import Iterable, Mapping
from dataclasses import dataclass, field
from typing import Any

from .errors import InvalidName, LockError

LOCK_FORMAT = 1
KINDS = ("ground", "air", "helo", "ship")
SYMBOL_LISTS = ("airbases", "flags", "groups", "menus", "templates", "zones")

_SCENARIO_RE = re.compile(r"[a-z][a-z0-9_]*")
_IDENT_RE = re.compile(r"[A-Z][A-Z0-9_]*")
_ROLE_RE = re.compile(r"[a-z][a-z0-9_-]*")


def check_scenario_name(name: str) -> str:
    if not isinstance(name, str) or not _SCENARIO_RE.fullmatch(name):
        raise InvalidName(f"scenario name {name!r} must match [a-z][a-z0-9_]* (e.g. 'op_north')")
    return name


def check_ident(what: str, ident: str) -> str:
    if not isinstance(ident, str) or not _IDENT_RE.fullmatch(ident):
        raise InvalidName(
            f"{what} ident {ident!r} must match [A-Z][A-Z0-9_]* (it becomes a Rust constant, e.g. 'SAM_NORTH')"
        )
    return ident


def check_role(role: str) -> str:
    if not isinstance(role, str) or not _ROLE_RE.fullmatch(role):
        raise InvalidName(f"role {role!r} must match [a-z][a-z0-9_-]* (e.g. 'sam', 'north')")
    return role


def canonical_roles(roles: Iterable[str] | None) -> list[str]:
    """Sorted, unique, checked roles. A plain string is one role, not a list of letters."""
    if roles is None:
        return []
    if isinstance(roles, str):
        roles = [roles]
    return sorted({check_role(r) for r in roles})


@dataclass(frozen=True)
class KindSymbol:
    ident: str
    kind: str
    roles: tuple[str, ...] = ()

    def to_json(self) -> dict[str, Any]:
        return {"ident": self.ident, "kind": self.kind, "roles": list(self.roles)}


@dataclass
class SymbolSet:
    airbases: list[str] = field(default_factory=list)
    flags: list[str] = field(default_factory=list)
    groups: list[KindSymbol] = field(default_factory=list)
    menus: list[str] = field(default_factory=list)
    templates: list[KindSymbol] = field(default_factory=list)
    zones: list[str] = field(default_factory=list)

    def to_json(self) -> dict[str, Any]:
        """The canonical JSON form: every list sorted by ident, roles sorted and unique."""

        def idents(xs: list[str]) -> list[dict[str, str]]:
            return [{"ident": i} for i in sorted(xs)]

        def kinds(xs: list[KindSymbol]) -> list[dict[str, Any]]:
            return [
                KindSymbol(s.ident, s.kind, tuple(sorted(set(s.roles)))).to_json()
                for s in sorted(xs, key=lambda s: s.ident)
            ]

        return {
            "airbases": idents(self.airbases),
            "flags": idents(self.flags),
            "groups": kinds(self.groups),
            "menus": idents(self.menus),
            "templates": kinds(self.templates),
            "zones": idents(self.zones),
        }

    @classmethod
    def from_json(cls, d: Mapping[str, Any]) -> SymbolSet:
        unknown = set(d) - set(SYMBOL_LISTS)
        if unknown:
            raise LockError(f"unknown symbol lists: {', '.join(sorted(unknown))}")

        def ident_list(name: str) -> list[str]:
            return [str(x["ident"]) for x in d.get(name, [])]

        def kind_list(name: str) -> list[KindSymbol]:
            return [
                KindSymbol(str(x["ident"]), str(x["kind"]), tuple(x.get("roles", [])))
                for x in d.get(name, [])
            ]

        return cls(
            airbases=ident_list("airbases"),
            flags=ident_list("flags"),
            groups=kind_list("groups"),
            menus=ident_list("menus"),
            templates=kind_list("templates"),
            zones=ident_list("zones"),
        )

    def check(self) -> None:
        """Names, kinds and uniqueness per symbol kind (the Rust lock check)."""
        lists: list[tuple[str, list[str]]] = [
            ("airbase", self.airbases),
            ("flag", self.flags),
            ("group", [s.ident for s in self.groups]),
            ("menu", self.menus),
            ("template", [s.ident for s in self.templates]),
            ("zone", self.zones),
        ]
        for what, idents in lists:
            seen: set[str] = set()
            for ident in idents:
                check_ident(what, ident)
                if ident in seen:
                    raise LockError(f"duplicate {what} symbol {ident!r}")
                seen.add(ident)
        for s in [*self.groups, *self.templates]:
            if s.kind not in KINDS:
                raise LockError(f"unknown kind {s.kind!r} for {s.ident!r} (expected {', '.join(KINDS)})")
            for r in s.roles:
                check_role(r)

    def keys(self) -> list[str]:
        """``"<kind>:<IDENT>"`` for every symbol, sorted (as in ``Attach.symbols_used``)."""
        out = [f"airbase:{i}" for i in self.airbases]
        out += [f"flag:{i}" for i in self.flags]
        out += [f"group:{s.ident}" for s in self.groups]
        out += [f"menu:{i}" for i in self.menus]
        out += [f"template:{s.ident}" for s in self.templates]
        out += [f"zone:{i}" for i in self.zones]
        return sorted(out)


def canonical_json(scenario: str, schema: int, symbols: SymbolSet) -> bytes:
    """The exact bytes that are hashed."""
    doc = {"scenario": scenario, "schema": int(schema), "symbols": symbols.to_json()}
    return json.dumps(doc, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode("utf-8")


def schema_sha256(scenario: str, schema: int, symbols: SymbolSet) -> str:
    return hashlib.sha256(canonical_json(scenario, schema, symbols)).hexdigest()


def circle_geometry(north: float, east: float, radius_m: float) -> dict[str, Any]:
    return {"circle": {"center": {"east": float(east) + 0.0, "north": float(north) + 0.0}, "radius_m": float(radius_m)}}


def polygon_geometry(points: Iterable[tuple[float, float]]) -> dict[str, Any]:
    return {"polygon": {"points": [{"east": float(e) + 0.0, "north": float(n) + 0.0} for n, e in points]}}


def lock_document(
    scenario: str,
    schema: int,
    symbols: SymbolSet,
    zones: Mapping[str, dict[str, Any]],
    generator: str,
) -> dict[str, Any]:
    check_scenario_name(scenario)
    symbols.check()
    return {
        "format": LOCK_FORMAT,
        "generator": generator,
        "geometry": {"zones": dict(zones)},
        "scenario": scenario,
        "schema": int(schema),
        "schema_sha256": schema_sha256(scenario, schema, symbols),
        "symbols": symbols.to_json(),
    }


def lock_text(lock: Mapping[str, Any]) -> str:
    """The file layout shared with ``ScenarioLock::to_json_pretty``."""
    return json.dumps(lock, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def parse_lock(text: str) -> dict[str, Any]:
    """Parses and verifies a lock file (format, names, kinds, hash)."""
    try:
        lock = json.loads(text)
    except json.JSONDecodeError as e:
        raise LockError(f"invalid scenario.lock.json: {e}") from e
    if not isinstance(lock, dict):
        raise LockError("invalid scenario.lock.json: not an object")
    if lock.get("format") != LOCK_FORMAT:
        raise LockError(f"unsupported scenario.lock.json format {lock.get('format')!r} (this sortie-gen understands {LOCK_FORMAT})")
    unknown = set(lock) - {"format", "generator", "geometry", "scenario", "schema", "schema_sha256", "symbols"}
    if unknown:
        raise LockError(f"invalid scenario.lock.json: unknown keys {', '.join(sorted(unknown))}")
    symbols = SymbolSet.from_json(lock.get("symbols", {}))
    check_scenario_name(lock.get("scenario", ""))
    symbols.check()
    computed = schema_sha256(lock["scenario"], int(lock["schema"]), symbols)
    if computed != lock.get("schema_sha256"):
        raise LockError(
            f"schema_sha256 mismatch: file says {lock.get('schema_sha256')}, symbols hash to {computed}"
        )
    return lock


@dataclass(frozen=True)
class LockDiff:
    """How a freshly computed lock differs from the file on disk."""

    hash_changed: bool
    geometry_changed: bool
    other_changed: bool
    added: tuple[str, ...]
    removed: tuple[str, ...]

    @property
    def is_empty(self) -> bool:
        return not (self.hash_changed or self.geometry_changed or self.other_changed)

    def describe(self) -> str:
        parts = []
        if self.added:
            parts.append("new symbols: " + ", ".join(self.added))
        if self.removed:
            parts.append("removed symbols: " + ", ".join(self.removed))
        if self.hash_changed and not (self.added or self.removed):
            parts.append("schema, kinds or roles changed")
        if self.geometry_changed:
            parts.append("zone geometry changed")
        if self.other_changed and not parts:
            parts.append("generator string changed")
        return "; ".join(parts) or "no changes"


def diff_locks(old: Mapping[str, Any] | None, new: Mapping[str, Any]) -> LockDiff:
    if old is None:
        keys = tuple(SymbolSet.from_json(new["symbols"]).keys())
        return LockDiff(True, True, True, keys, ())
    old_keys = set(SymbolSet.from_json(old.get("symbols", {})).keys())
    new_keys = set(SymbolSet.from_json(new["symbols"]).keys())
    return LockDiff(
        hash_changed=old.get("schema_sha256") != new["schema_sha256"],
        geometry_changed=old.get("geometry") != new["geometry"],
        other_changed=lock_text(old) != lock_text(new),
        added=tuple(sorted(new_keys - old_keys)),
        removed=tuple(sorted(old_keys - new_keys)),
    )
