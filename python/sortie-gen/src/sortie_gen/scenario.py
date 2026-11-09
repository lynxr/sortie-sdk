"""``Scenario``: a thin wrapper over ``dcs.Mission`` that knows about Sortie symbols.

A scenario *declares* zones, groups, templates, airbases, player flights, flags and menus. The
declarations are checked immediately (names, idents, roles, uniqueness: pydcs checks none of
it, and DCS silently destroys a group whose name is reused), and every :meth:`Scenario.build`
replays them into a fresh ``dcs.Mission`` on a fresh terrain, so building twice gives
byte-identical files and never leaks state (parking slots, ids, dictionary keys) between builds.

Outputs of a build (see :class:`Build`): the ``.miz`` with its bootstrap trigger,
``manifest.pb`` (+ ``manifest.json``) and the ``scenario.lock.json`` document.
"""

from __future__ import annotations

import difflib
import enum
import hashlib
import inspect
import json
import math
import platform
import re
import sys
import warnings
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

from . import _pydcs
from . import lock as lockmod
from . import miz as mizmod
from . import ulid
from .errors import InvalidName, NameConflict, ScenarioError

import dcs  # noqa: E402
import dcs.mapping  # noqa: E402
from dcs.action import SetFlagValue  # noqa: E402
from dcs.mission import StartType as PydcsStartType  # noqa: E402
from dcs.triggers import TriggerStart  # noqa: E402
from dcs.unit import Skill  # noqa: E402

from ._proto.sortie.manifest.v1 import manifest_pb2 as mpb  # noqa: E402
from ._proto.sortie.v1 import common_pb2 as cpb  # noqa: E402

__all__ = [
    "Build",
    "Coalition",
    "Scenario",
    "SortieWarning",
    "StartType",
    "TypeUse",
]

GENERATOR_NAME = "sortie-gen"
MAX_NAME_BYTES = 128
RUNTIME_SEPARATOR = "#"
FLAGS_TRIGGER_COMMENT = "sortie flags"


def _version() -> str:
    from . import __version__

    return __version__


class SortieWarning(UserWarning):
    """Something in the scenario works but is probably not what the author wants."""


class StartType(enum.Enum):
    """How a flight starts (maps to ``sortie.v1.StartType`` and to pydcs's start types)."""

    Runway = "runway"
    Hot = "hot"
    Cold = "cold"
    Air = "air"

    @property
    def proto(self) -> int:
        return {
            StartType.Runway: cpb.START_TYPE_RUNWAY,
            StartType.Hot: cpb.START_TYPE_HOT,
            StartType.Cold: cpb.START_TYPE_COLD,
            StartType.Air: cpb.START_TYPE_AIR,
        }[self]


class Coalition(enum.Enum):
    Neutral = "neutral"
    Red = "red"
    Blue = "blue"

    @property
    def proto(self) -> int:
        return {
            Coalition.Neutral: cpb.COALITION_NEUTRAL,
            Coalition.Red: cpb.COALITION_RED,
            Coalition.Blue: cpb.COALITION_BLUE,
        }[self]

    @classmethod
    def parse(cls, value: Coalition | str) -> Coalition:
        if isinstance(value, Coalition):
            return value
        v = str(value).strip().lower()
        if v in ("neutrals", "neutral"):
            return Coalition.Neutral
        try:
            return Coalition(v)
        except ValueError:
            raise ScenarioError(f"unknown coalition {value!r} (use 'red', 'blue' or 'neutral')") from None


# ----- small parsers -------------------------------------------------------------------------

_DURATION_RE = re.compile(r"\s*(\d+(?:\.\d+)?)\s*(h|m|s)\s*", re.IGNORECASE)


def parse_duration(value: Any) -> int:
    """Seconds from ``"3h"``, ``"1h30m"``, ``"90m"``, ``"45s"``, a number of seconds or a timedelta.

    ``None`` and ``0`` mean "no limit".
    """
    if value is None:
        return 0
    if isinstance(value, bool):
        raise ScenarioError("a duration cannot be a bool")
    if isinstance(value, timedelta):
        seconds = value.total_seconds()
    elif isinstance(value, (int, float)):
        seconds = float(value)
    elif isinstance(value, str):
        text = value.strip()
        if not text:
            return 0
        pos, seconds = 0, 0.0
        for m in _DURATION_RE.finditer(text):
            if m.start() != pos:
                break
            seconds += float(m.group(1)) * {"h": 3600, "m": 60, "s": 1}[m.group(2).lower()]
            pos = m.end()
        if pos != len(text):
            raise ScenarioError(f"cannot read the duration {value!r} (examples: '3h', '1h30m', '90m', '45s')")
    else:
        raise ScenarioError(f"cannot read the duration {value!r}")
    if seconds < 0 or not math.isfinite(seconds) or seconds >= 2**32:
        raise ScenarioError(f"a duration must be between 0 and 2^32 seconds, not {value!r}")
    return int(round(seconds))


def parse_start(value: Any, utc_offset: timezone) -> datetime:
    """An aware UTC datetime. Strings are RFC 3339; a time without an offset is theatre-local."""
    if value is None:
        raise ScenarioError("Scenario(start=...) is required, e.g. start='2026-11-01T06:30:00Z'")
    if isinstance(value, str):
        text = value.strip()
        try:
            dt = datetime.fromisoformat(text[:-1] + "+00:00" if text.endswith(("Z", "z")) else text)
        except ValueError:
            raise ScenarioError(f"start={value!r} is not an RFC 3339 date-time like '2026-11-01T06:30:00Z'") from None
    elif isinstance(value, datetime):
        dt = value
    else:
        raise ScenarioError(f"start must be a string or a datetime, not {type(value).__name__}")
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=utc_offset)
    return dt.astimezone(timezone.utc).replace(microsecond=0)


def check_dcs_name(what: str, name: Any, *, group: bool = False) -> str:
    """The name rules shared with ``sortie_manifest::naming`` (``#`` is reserved for spawns)."""
    if not isinstance(name, str) or not name:
        raise InvalidName(f"{what} name must be a non-empty string, not {name!r}")
    if len(name.encode("utf-8")) > MAX_NAME_BYTES:
        raise InvalidName(f"{what} name {name!r} is longer than {MAX_NAME_BYTES} bytes")
    if name.strip() != name:
        raise InvalidName(f"{what} name {name!r} starts or ends with whitespace")
    if any(ord(c) < 32 or ord(c) == 127 for c in name):
        raise InvalidName(f"{what} name {name!r} contains a control character")
    if group and RUNTIME_SEPARATOR in name:
        raise InvalidName(
            f"{what} name {name!r} contains '#', which is reserved for spawned groups (<template>#<session4>-<n>)"
        )
    return name


def unit_name(group_name: str, k: int) -> str:
    """``<group>-<k>``, the unit naming rule of ``sortie_manifest::naming`` (k from 1)."""
    return f"{group_name}-{k}"


def _skill(value: Skill | str) -> Skill:
    if isinstance(value, Skill):
        return value
    try:
        return Skill(str(value))
    except ValueError:
        names = ", ".join(s.value for s in Skill)
        raise ScenarioError(f"unknown skill {value!r} (one of {names})") from None


def _clean(x: float) -> float:
    """Turns -0.0 into 0.0 (protobuf runtimes disagree on whether -0.0 is a default value)."""
    return float(x) + 0.0


def _size(what: str, size: Any, unit_type: Any) -> int:
    top = int(getattr(unit_type, "group_size_max", 4) or 4)
    if isinstance(size, bool) or not isinstance(size, int) or not 1 <= size <= top:
        raise ScenarioError(f"{what}: size must be 1..{top} for {unit_type.id}, not {size!r}")
    return size


# ----- handles returned to the scenario author ----------------------------------------------


@dataclass(frozen=True)
class ZoneHandle:
    name: str
    ident: str | None
    center: dcs.mapping.Point
    radius: float | None
    points: tuple[dcs.mapping.Point, ...] = ()


@dataclass(frozen=True)
class GroupHandle:
    name: str
    ident: str | None
    kind: str
    roles: tuple[str, ...]
    country: str
    units: tuple[str, ...]


@dataclass(frozen=True)
class TemplateHandle:
    name: str
    ident: str
    kind: str
    roles: tuple[str, ...]
    country: str
    type_name: str
    size: int


@dataclass(frozen=True)
class AirbaseHandle:
    name: str
    ident: str
    id: int
    position: dcs.mapping.Point


@dataclass(frozen=True)
class FlightHandle:
    name: str
    type_name: str
    country: str
    airbase: str
    start: StartType
    size: int
    units: tuple[str, ...]


@dataclass(frozen=True)
class FlagHandle:
    ident: str
    name: str
    initial: int


@dataclass(frozen=True)
class MenuHandle:
    ident: str
    label: str
    path: tuple[str, ...]


# ----- declarations (replayed into a fresh Mission by every build) ---------------------------


@dataclass
class _Zone:
    handle: ZoneHandle
    hidden: bool
    color: tuple[float, float, float, float] | None


@dataclass
class _Ground:
    handle: GroupHandle
    at: tuple[float, float]
    unit_types: list[Any]
    heading: float
    formation: str
    spacing: float
    skill: Skill
    late_activation: bool
    hidden: bool


@dataclass
class _Template:
    handle: TemplateHandle
    unit_type: Any
    task: Any
    at: tuple[float, float] | None
    airbase: str | None
    altitude: float
    speed: float | None
    skill: Skill
    payload: Any


@dataclass
class _Airbase:
    handle: AirbaseHandle
    coalition: Coalition | None


@dataclass
class _Flight:
    handle: FlightHandle
    unit_type: Any
    task: Any
    skill: Skill
    payload: Any
    at: tuple[float, float] | None
    altitude: float


_FORMATIONS = ("line", "star", "rectangle", "vee")


# ----- build result --------------------------------------------------------------------------


@dataclass(frozen=True)
class TypeUse:
    """A DCS type name a build uses and where: what ``Generator.ValidateTypes`` checks.

    ``static`` marks a static object (``StaticObject.getDescByName`` in DCS); everything else
    is a unit of a group or template (``Unit.getDescByName``).
    """

    type_name: str
    where: str
    static: bool = False


@dataclass
class Build:
    """Everything one build produced. Nothing is written until :meth:`write`."""

    build_id: str
    manifest: mpb.Manifest
    manifest_pb: bytes
    manifest_json: str
    digest: str
    miz: bytes
    lock: dict[str, Any]
    warnings: list[str] = field(default_factory=list)
    #: Every unit and static type of the mission and its templates, first use first.
    type_uses: list[TypeUse] = field(default_factory=list)

    def type_names(self) -> list[str]:
        """The distinct type names of :attr:`type_uses`, sorted."""
        return sorted({u.type_name for u in self.type_uses})

    @property
    def lock_text(self) -> str:
        return lockmod.lock_text(self.lock)

    @property
    def bootstrap(self) -> str:
        return mizmod.bootstrap_script(self.manifest.scenario, self.build_id, self.digest, self.manifest.schema)

    def file_names(self, stem: str | None = None) -> dict[str, str]:
        s = stem or self.build_id
        return {"miz": f"{s}.miz", "pb": f"{s}.manifest.pb", "json": f"{s}.manifest.json"}

    def write(self, out_dir: str | Path, *, stem: str | None = None, miz: bool = True) -> dict[str, Path]:
        """Atomically writes ``<stem>.manifest.pb``, ``<stem>.manifest.json`` and ``<stem>.miz``."""
        from .fsutil import atomic_write_bytes, atomic_write_text

        out = Path(out_dir)
        names = self.file_names(stem)
        written: dict[str, Path] = {}
        # The manifest first: a .miz is only useful next to its manifest.
        written["pb"] = atomic_write_bytes(out / names["pb"], self.manifest_pb)
        written["json"] = atomic_write_text(out / names["json"], self.manifest_json)
        if miz:
            written["miz"] = atomic_write_bytes(out / names["miz"], self.miz)
        return written


# ----- Scenario ------------------------------------------------------------------------------


class Scenario:
    """A Sortie scenario on top of pydcs (see the module docs and the generator guide)."""

    def __init__(
        self,
        name: str,
        *,
        schema: int,
        theatre: Any,
        start: str | datetime | None = None,
        time_limit: Any = None,
        description: str | None = None,
        source: str | Path | None = None,
    ) -> None:
        self.name = lockmod.check_scenario_name(name)
        if isinstance(schema, bool) or not isinstance(schema, int) or schema < 1:
            raise ScenarioError(f"schema must be an integer >= 1, not {schema!r}")
        self.schema = schema
        if theatre is None or not hasattr(theatre, "airports") or not hasattr(theatre, "utc_offset"):
            raise ScenarioError("theatre must be a pydcs terrain, e.g. dcs.terrain.caucasus.Caucasus()")
        self.theatre = theatre
        self.start = parse_start(start, theatre.utc_offset)
        self._time_limit_s = parse_duration(time_limit)
        self.description = description or f"Sortie scenario {name}"
        if source is None:
            source = inspect.stack()[1].frame.f_globals.get("__file__")
        self.source: Path | None = Path(source).resolve() if source else None

        self._decls: list[Any] = []
        self._zones: dict[str, _Zone] = {}
        self._templates: dict[str, _Template] = {}
        self._airbases: dict[str, _Airbase] = {}
        self._flags: dict[str, FlagHandle] = {}
        self._menus: dict[str, MenuHandle] = {}
        self._group_names: set[str] = set()
        self._unit_names: set[str] = set()
        self._idents: dict[str, set[str]] = {k: set() for k in ("group", "template", "zone", "airbase", "flag", "menu")}
        self._hooks: list[Callable[[dcs.Mission], Any]] = []
        self._probe = dcs.Mission(theatre)

    def __repr__(self) -> str:
        return f"Scenario({self.name!r}, schema={self.schema}, theatre={self.theatre.name})"

    # ----- positions -----------------------------------------------------------------------

    def latlon(self, lat: float, lon: float) -> dcs.mapping.Point:
        """A map point from WGS84 latitude/longitude (pydcs projection of the theatre)."""
        if not (-90.0 <= lat <= 90.0 and -180.0 <= lon <= 180.0):
            raise ScenarioError(f"latitude/longitude out of range: {lat}, {lon}")
        return _pydcs.point_from_latlon(self.theatre, lat, lon)

    def point(self, north: float, east: float) -> dcs.mapping.Point:
        """A map point from mission-editor coordinates (x = north, y = east, metres)."""
        return _pydcs.point(self.theatre, north, east)

    def _xy(self, at: Any, what: str) -> tuple[float, float]:
        if isinstance(at, ZoneHandle):
            at = at.center
        if isinstance(at, AirbaseHandle):
            at = at.position
        if isinstance(at, dcs.mapping.LatLng):
            at = self.latlon(at.lat, at.lng)
        if isinstance(at, dcs.mapping.Point):
            x, y = float(at.x), float(at.y)
            if not (math.isfinite(x) and math.isfinite(y)):
                raise ScenarioError(f"{what}: the position is not finite")
            return (x, y)
        raise ScenarioError(
            f"{what}: pass a position made with sc.latlon(lat, lon), sc.point(north, east), "
            f"a zone or an airbase (a bare tuple is ambiguous), not {at!r}"
        )

    # ----- bookkeeping -----------------------------------------------------------------------

    def _ident(self, what: str, ident: str | None, *, required: bool) -> str | None:
        if ident is None:
            if required:
                raise ScenarioError(f"a {what} needs an ident (the Rust constant), e.g. ident='RED_CAP'")
            return None
        lockmod.check_ident(what, ident)
        if ident in self._idents[what]:
            raise NameConflict(f"{what} ident {ident!r} is declared twice")
        return ident

    def _claim_ident(self, what: str, ident: str | None) -> None:
        if ident is not None:
            self._idents[what].add(ident)

    def _group_name(self, what: str, name: str) -> str:
        check_dcs_name(what, name, group=True)
        if name in self._group_names:
            raise NameConflict(
                f"a group named {name!r} already exists in this scenario (DCS would destroy one of them); choose another name"
            )
        if name in self._templates:
            raise NameConflict(f"{name!r} is already the name of a template; groups and templates share one namespace")
        return name

    def _unit_names_for(self, group: str, count: int) -> list[str]:
        names = [unit_name(group, k) for k in range(1, count + 1)]
        for n in names:
            check_dcs_name("unit", n)
            if n in self._unit_names:
                raise NameConflict(f"a unit named {n!r} already exists in this scenario; rename group {group!r}")
        return names

    def _country(self, country: Any, what: str) -> str:
        name = getattr(country, "name", country)
        if not isinstance(name, str):
            raise ScenarioError(f"{what}: country must be a pydcs country name like 'Russia', not {country!r}")
        if self._probe.country(name) is None:
            known = sorted(c for col in self._probe.coalition.values() for c in col.countries)
            hint = difflib.get_close_matches(name, known, n=3)
            extra = f" (did you mean {', '.join(map(repr, hint))}?)" if hint else ""
            raise ScenarioError(f"{what}: unknown country {name!r}{extra}")
        return name

    def _airport(self, airbase: Any, what: str) -> Any:
        name = getattr(airbase, "name", airbase)
        airports = self.theatre.airports
        if isinstance(name, str) and name in airports:
            return airports[name]
        hint = difflib.get_close_matches(str(name), list(airports), n=3)
        extra = f" (did you mean {', '.join(map(repr, hint))}?)" if hint else ""
        raise ScenarioError(f"{what}: {self.theatre.name} has no airbase {name!r}{extra}")

    # ----- declarations ----------------------------------------------------------------------

    def zone(
        self,
        name: str,
        *,
        ident: str | None = None,
        at: Any = None,
        radius: float | None = None,
        points: Sequence[Any] | None = None,
        hidden: bool = True,
        color: tuple[float, float, float, float] | None = None,
    ) -> ZoneHandle:
        """A trigger zone: a circle (``at`` + ``radius`` in metres) or a quad (four ``points``).

        Without an ident the zone is only written to the ``.miz`` (no Rust handle)."""
        check_dcs_name("zone", name)
        if name in self._zones:
            raise NameConflict(f"zone {name!r} is declared twice")
        ident = self._ident("zone", ident, required=False)
        if points is not None:
            if at is not None or radius is not None:
                raise ScenarioError(f"zone {name!r}: give either at+radius or points, not both")
            pts = [self._xy(p, f"zone {name!r}") for p in points]
            if len(pts) != 4:
                raise ScenarioError(f"zone {name!r}: DCS quad zones have exactly 4 points, got {len(pts)}")
            cx = sum(p[0] for p in pts) / 4
            cy = sum(p[1] for p in pts) / 4
            handle = ZoneHandle(name, ident, self.point(cx, cy), None, tuple(self.point(*p) for p in pts))
        else:
            if at is None or radius is None:
                raise ScenarioError(f"zone {name!r}: a circle zone needs at= and radius=")
            r = float(radius)
            if not (math.isfinite(r) and r > 0):
                raise ScenarioError(f"zone {name!r}: radius must be > 0 metres")
            x, y = self._xy(at, f"zone {name!r}")
            handle = ZoneHandle(name, ident, self.point(x, y), r)
        self._claim_ident("zone", ident)
        decl = _Zone(handle, bool(hidden), color)
        self._zones[name] = decl
        self._decls.append(decl)
        return handle

    def ground_group(
        self,
        name: str,
        *,
        ident: str | None = None,
        country: Any,
        at: Any,
        units: Sequence[Any],
        heading: float = 0.0,
        formation: str = "star",
        spacing: float = 40.0,
        skill: Skill | str = "Excellent",
        roles: Iterable[str] | None = None,
        late_activation: bool = False,
        hidden: bool = False,
    ) -> GroupHandle:
        """A ground group placed in the ``.miz``; unit ``k`` is named ``<name>-<k>``.

        ``formation`` is a pydcs formation (``line``, ``star``, ``rectangle``, ``vee``) with
        ``spacing`` metres between units; the first unit stands at ``at``.
        """
        self._group_name("group", name)
        ident = self._ident("group", ident, required=False)
        country = self._country(country, f"group {name!r}")
        types = list(units)
        if not types:
            raise ScenarioError(f"group {name!r} needs at least one unit")
        for t in types:
            if not _pydcs.is_vehicle_type(t):
                raise ScenarioError(f"group {name!r}: {t!r} is not a pydcs vehicle type (dcs.vehicles.*)")
        if formation not in _FORMATIONS:
            raise ScenarioError(f"group {name!r}: formation must be one of {', '.join(_FORMATIONS)}")
        spacing_ = float(spacing)
        if not (math.isfinite(spacing_) and spacing_ > 0):
            raise ScenarioError(f"group {name!r}: spacing must be > 0 metres")
        roles_ = tuple(lockmod.canonical_roles(roles))
        unit_names = self._unit_names_for(name, len(types))
        handle = GroupHandle(name, ident, "ground", roles_, country, tuple(unit_names))
        decl = _Ground(
            handle=handle,
            at=self._xy(at, f"group {name!r}"),
            unit_types=types,
            heading=float(heading),
            formation=formation,
            spacing=spacing_,
            skill=_skill(skill),
            late_activation=bool(late_activation),
            hidden=bool(hidden),
        )
        self._claim_ident("group", ident)
        self._group_names.add(name)
        self._unit_names.update(unit_names)
        self._decls.append(decl)
        return handle

    def air_template(
        self,
        name: str,
        unit_type: Any,
        *,
        ident: str,
        country: Any,
        size: int = 1,
        task: Any = None,
        at: Any = None,
        airbase: Any = None,
        altitude: float | None = None,
        speed: float | None = None,
        skill: Skill | str = "High",
        roles: Iterable[str] | None = None,
        payload: Any = None,
    ) -> TemplateHandle:
        """An aircraft group kept out of the ``.miz`` and written to the manifest for spawning.

        The logic spawns it with ``ctx.spawn_group(templates::IDENT)``; the engine builds the
        ``coalition.addGroup`` table from ``dcs_group_json`` (pydcs's group table: payload,
        radio, task) and names the group ``<name>#<session4>-<n>`` unless ``.named()`` is used.
        Default placement (for spawns without ``.at``/``.from_airbase``): ``at``, else the
        ``airbase``, else the first airbase declared in the scenario, else the coalition's
        bullseye; ``altitude`` defaults to 6000 m (2000 m for helicopters), ``speed`` (km/h) to
        pydcs's default. ``payload`` is a pydcs loadout name or a list of pylon tuples.
        """
        check_dcs_name("template", name, group=True)
        if name in self._templates:
            raise NameConflict(f"template {name!r} is declared twice")
        if name in self._group_names:
            raise NameConflict(f"{name!r} is already the name of a group; groups and templates share one namespace")
        ident_ = self._ident("template", ident, required=True)
        country = self._country(country, f"template {name!r}")
        if not _pydcs.is_flying_type(unit_type):
            raise ScenarioError(
                f"template {name!r}: {unit_type!r} is not a pydcs aircraft type (dcs.planes.*, dcs.helicopters.*)"
            )
        size = _size(f"template {name!r}", size, unit_type)
        maintask = task or unit_type.task_default
        if maintask is None:
            raise ScenarioError(f"template {name!r}: {unit_type.id} has no default task; pass task=dcs.task.CAP or similar")
        helo = bool(unit_type.helicopter)
        at_xy = self._xy(at, f"template {name!r}") if at is not None else None
        airbase_name = self._airport(airbase, f"template {name!r}").name if airbase is not None else None
        roles_ = tuple(lockmod.canonical_roles(roles))
        assert ident_ is not None
        handle = TemplateHandle(name, ident_, "helo" if helo else "air", roles_, country, unit_type.id, size)
        decl = _Template(
            handle=handle,
            unit_type=unit_type,
            task=maintask,
            at=at_xy,
            airbase=airbase_name,
            altitude=float(altitude if altitude is not None else (2000.0 if helo else 6000.0)),
            speed=float(speed) if speed is not None else None,
            skill=_skill(skill),
            payload=payload,
        )
        self._claim_ident("template", ident_)
        self._templates[name] = decl
        self._decls.append(decl)
        return handle

    def airbase(self, name: Any, *, ident: str, coalition: Coalition | str | None = None) -> AirbaseHandle:
        """Declares an airbase symbol (``airbases::IDENT``) for the logic.

        ``coalition`` sets the airbase's owner in the mission; without it the theatre's default
        owner is kept (neutral in pydcs): declaring a symbol never changes the mission by itself.
        The exception is an airbase a ``player_flight`` starts from on the ground (Cold, Hot or
        Runway): it belongs to that flight's coalition (multiplayer slots need it, see
        ``player_flight``), and with a different ``coalition`` here the build fails.
        """
        airport = self._airport(name, f"airbase {name!r}")
        if airport.name in self._airbases:
            raise NameConflict(f"airbase {airport.name!r} is declared twice")
        ident_ = self._ident("airbase", ident, required=True)
        assert ident_ is not None
        handle = AirbaseHandle(airport.name, ident_, int(airport.id), self.point(airport.position.x, airport.position.y))
        decl = _Airbase(handle, Coalition.parse(coalition) if coalition is not None else None)
        self._claim_ident("airbase", ident_)
        self._airbases[airport.name] = decl
        self._decls.append(decl)
        return handle

    def player_flight(
        self,
        name: str,
        unit_type: Any,
        *,
        country: Any,
        airbase: Any,
        start: StartType = StartType.Cold,
        size: int = 1,
        task: Any = None,
        skill: Skill | str = "Client",
        payload: Any = None,
        at: Any = None,
        altitude: float = 3000.0,
    ) -> FlightHandle:
        """Client slots: a flight at ``airbase`` (parking for Cold/Hot, the runway for Runway;
        ``Air`` starts at ``at``/``altitude``). Units are named ``<name>-<k>``.
        Player flights go to ``manifest.player_flights``, not to the lock.

        A Cold, Hot or Runway start gives ``airbase`` to the flight's coalition: in multiplayer
        DCS offers a parking slot only to the coalition that owns the airfield, so a slot at a
        neutral or enemy airfield could not be taken. The build fails when the scenario declares
        that airbase with another ``coalition``, or starts flights of two coalitions there."""
        self._group_name("player flight", name)
        country = self._country(country, f"player flight {name!r}")
        if not _pydcs.is_flying_type(unit_type):
            raise ScenarioError(f"player flight {name!r}: {unit_type!r} is not a pydcs aircraft type")
        if not getattr(unit_type, "flyable", False):
            raise ScenarioError(f"player flight {name!r}: {unit_type.id} is not flyable by players")
        if not isinstance(start, StartType):
            raise ScenarioError(f"player flight {name!r}: start must be a sortie_gen.StartType, not {start!r}")
        size = _size(f"player flight {name!r}", size, unit_type)
        airport = self._airport(airbase, f"player flight {name!r}")
        if start is StartType.Air and at is None:
            raise ScenarioError(f"player flight {name!r}: an air start needs at=")
        at_xy = self._xy(at, f"player flight {name!r}") if at is not None else None
        unit_names = self._unit_names_for(name, size)
        handle = FlightHandle(name, unit_type.id, country, airport.name, start, size, tuple(unit_names))
        decl = _Flight(handle, unit_type, task, _skill(skill), payload, at_xy, float(altitude))
        self._group_names.add(name)
        self._unit_names.update(unit_names)
        self._decls.append(decl)
        return handle

    def flag(self, ident: str, *, name: str | None = None, initial: int | bool = 0) -> FlagHandle:
        """A user flag (``flags::IDENT``); its DCS name defaults to the ident."""
        ident_ = self._ident("flag", ident, required=True)
        assert ident_ is not None
        dcs_name = check_dcs_name("flag", name if name is not None else ident_)
        if any(f.name == dcs_name for f in self._flags.values()):
            raise NameConflict(f"flag name {dcs_name!r} is used twice")
        value = int(initial)
        if not -(2**31) <= value < 2**31:
            raise ScenarioError(f"flag {ident!r}: initial value out of the int32 range")
        handle = FlagHandle(ident_, dcs_name, value)
        self._claim_ident("flag", ident_)
        self._flags[ident_] = handle
        self._decls.append(handle)
        return handle

    def menu(self, ident: str, label: str, *, path: Sequence[str] = ()) -> MenuHandle:
        """An F10 menu command (``menus::IDENT``); the engine creates it when the logic asks."""
        ident_ = self._ident("menu", ident, required=True)
        assert ident_ is not None
        if not isinstance(label, str) or not label.strip():
            raise ScenarioError(f"menu {ident!r}: the label must not be empty")
        if isinstance(path, str):
            path = [path]
        path_ = tuple(str(p) for p in path)
        if any(not p.strip() for p in path_):
            raise ScenarioError(f"menu {ident!r}: sub-menu names must not be empty")
        handle = MenuHandle(ident_, label, path_)
        self._claim_ident("menu", ident_)
        self._menus[ident_] = handle
        self._decls.append(handle)
        return handle

    @property
    def time_limit(self) -> int:
        """Session time limit in seconds of model time (0 = none); accepts ``"3h"`` etc."""
        return self._time_limit_s

    @time_limit.setter
    def time_limit(self, value: Any) -> None:
        self._time_limit_s = parse_duration(value)

    def pydcs(self, fn: Callable[[dcs.Mission], Any]) -> Callable[[dcs.Mission], Any]:
        """Decorator: ``fn(mission)`` runs on every build after Sortie's declarations, for what
        Sortie does not wrap (statics, waypoints, weather). Groups it adds appear in the
        manifest without an ident; all names are checked for uniqueness afterwards."""
        self._hooks.append(fn)
        return fn

    # ----- symbols and lock -------------------------------------------------------------------

    def symbols(self) -> lockmod.SymbolSet:
        s = lockmod.SymbolSet()
        for d in self._decls:
            if isinstance(d, _Zone) and d.handle.ident:
                s.zones.append(d.handle.ident)
            elif isinstance(d, _Ground) and d.handle.ident:
                s.groups.append(lockmod.KindSymbol(d.handle.ident, d.handle.kind, d.handle.roles))
            elif isinstance(d, _Template):
                s.templates.append(lockmod.KindSymbol(d.handle.ident, d.handle.kind, d.handle.roles))
            elif isinstance(d, _Airbase):
                s.airbases.append(d.handle.ident)
            elif isinstance(d, FlagHandle):
                s.flags.append(d.ident)
            elif isinstance(d, MenuHandle):
                s.menus.append(d.ident)
        return s

    def schema_sha256(self) -> str:
        return lockmod.schema_sha256(self.name, self.schema, self.symbols())

    def generator_string(self) -> str:
        return f"{GENERATOR_NAME} {_version()}"

    def lock(self) -> dict[str, Any]:
        """The ``scenario.lock.json`` document (see :mod:`sortie_gen.lock`)."""
        geometry: dict[str, Any] = {}
        for d in self._decls:
            if isinstance(d, _Zone) and d.handle.ident:
                h = d.handle
                if h.radius is not None:
                    geometry[h.ident] = lockmod.circle_geometry(h.center.x, h.center.y, h.radius)
                else:
                    geometry[h.ident] = lockmod.polygon_geometry((p.x, p.y) for p in h.points)
        return lockmod.lock_document(self.name, self.schema, self.symbols(), geometry, self.generator_string())

    def lock_text(self) -> str:
        return lockmod.lock_text(self.lock())

    def base_dir(self) -> Path:
        return self.source.parent if self.source else Path.cwd()

    def default_lock_path(self) -> Path:
        return self.base_dir() / "scenario.lock.json"

    def default_out_dir(self) -> Path:
        return self.base_dir() / "dist"

    # ----- build ------------------------------------------------------------------------------

    def build(self, build_id: str | None = None) -> Build:
        """Builds the mission, the manifest and the lock in memory.

        ``build_id`` is the engine's (``Generator.BeginBuild``); without one a content-derived
        ULID is used: same scenario, same id, byte-identical files.
        """
        if build_id is not None and not ulid.is_ulid(build_id):
            raise ScenarioError(f"build id {build_id!r} is not a ULID")
        _pydcs.mode()
        with warnings.catch_warnings(record=True) as records:
            warnings.simplefilter("always", SortieWarning)
            builder = _MissionBuilder(self)
            mission = builder.run()
            manifest = builder.manifest()
            type_uses = builder.type_uses()
        caught = []
        for r in records:
            if issubclass(r.category, SortieWarning):
                caught.append(str(r.message))
                warnings.warn(str(r.message), SortieWarning, stacklevel=2)
            else:
                warnings.warn_explicit(r.message, r.category, r.filename, r.lineno)

        if build_id is None:
            # Same scenario, same id: the manifest plus the mission content before the bootstrap
            # (which carries the id). A change made only by a @sc.pydcs hook changes the id too.
            manifest.build_id = ""
            content = manifest.SerializeToString(deterministic=True) + mizmod.content_sha256(
                mizmod.mission_bytes(mission)
            )
            build_id = ulid.content_id(self.start, content)
        manifest.build_id = build_id
        manifest_pb = manifest.SerializeToString(deterministic=True)
        digest = hashlib.sha256(manifest_pb).hexdigest()

        script = mizmod.bootstrap_script(self.name, build_id, digest, self.schema)
        mizmod.add_start_script(mission, script)
        miz_bytes = mizmod.mission_bytes(mission)

        from .manifest import manifest_json

        return Build(
            build_id=build_id,
            manifest=manifest,
            manifest_pb=manifest_pb,
            manifest_json=manifest_json(manifest),
            digest=digest,
            miz=miz_bytes,
            lock=self.lock(),
            warnings=caught,
            type_uses=type_uses,
        )

    def cli(self, argv: Sequence[str] | None = None) -> None:
        """``python scenario.py lock | build | register | launch | watch | results``."""
        from .cli import scenario_main

        sys.exit(scenario_main(self, argv))


# ----- the per-build mission builder ---------------------------------------------------------


_COALITION_OF_SIDE = {"blue": Coalition.Blue, "red": Coalition.Red, "neutrals": Coalition.Neutral}
_START_OF_POINT = {
    "TakeOffParking": StartType.Cold,
    "TakeOffParkingHot": StartType.Hot,
    "TakeOff": StartType.Runway,
}


class _MissionBuilder:
    """Replays the declarations of a scenario into a fresh mission and scans it back."""

    def __init__(self, sc: Scenario) -> None:
        self.sc = sc
        self.terrain = type(sc.theatre)()
        self.mission = dcs.Mission(self.terrain)
        # Templates live in their own mission so that they take no group/unit ids of the .miz.
        self.scratch = dcs.Mission(self.terrain)
        self.templates: list[tuple[_Template, Any]] = []
        self.airbases: list[_Airbase] = []
        self.flights: dict[str, _Flight] = {}
        self.grounds: dict[str, _Ground] = {}

    # -- mission ----------------------------------------------------------------------------

    def run(self) -> dcs.Mission:
        sc, m = self.sc, self.mission
        m.start_time = sc.start.astimezone(self.terrain.utc_offset)
        m.set_sortie_text(sc.name)
        m.set_description_text(sc.description)
        self._own_player_airfields()
        for d in sc._decls:
            if isinstance(d, _Zone):
                self._zone(d)
            elif isinstance(d, _Ground):
                self._ground(d)
            elif isinstance(d, _Airbase):
                self._airbase(d)
            elif isinstance(d, _Flight):
                self._flight(d)
        # Templates last: their default placement may use any declared airbase.
        for d in sc._decls:
            if isinstance(d, _Template):
                self.templates.append((d, self._template(d)))
        initial = [f for f in sc._flags.values() if f.initial != 0]
        if initial:
            trig = TriggerStart(comment=FLAGS_TRIGGER_COMMENT)
            for f in initial:
                trig.add_action(SetFlagValue(f.name, f.initial))
            m.triggerrules.triggers.append(trig)
        for hook in sc._hooks:
            hook(m)
        self._warn_unowned_player_airfields()
        self._check_names()
        return m

    def _warn_unowned_player_airfields(self) -> None:
        """After the hooks: player groups starting on the ground at an airfield their coalition
        does not own (a hook added them or changed the owner) cannot be taken in multiplayer."""
        for coalition, _country, g, _cat in self._placed_groups():
            if not any(_pydcs.skill_name(getattr(u, "skill", None)) in ("Client", "Player") for u in g.units):
                continue
            first = g.points[0] if g.points else None
            aid = getattr(first, "airdrome_id", None)
            if getattr(first, "type", "") not in _START_OF_POINT or not aid:
                continue
            ap = self.terrain.airport_by_id(aid)
            if ap is not None and Coalition.parse(ap.coalition) is not coalition:
                warnings.warn(
                    f"player group {g.name!r} ({coalition.value}) starts at {ap.name}, which belongs to "
                    f"{Coalition.parse(ap.coalition).value}: in multiplayer DCS offers parking slots only "
                    "to the coalition that owns the airfield",
                    SortieWarning,
                    stacklevel=2,
                )

    def _zone(self, d: _Zone) -> None:
        h = d.handle
        pos = _pydcs.point(self.terrain, h.center.x, h.center.y)
        if h.radius is not None:
            self.mission.triggers.add_triggerzone(pos, h.radius, d.hidden, h.name, d.color)
        else:
            pts = [_pydcs.point(self.terrain, p.x, p.y) for p in h.points]
            self.mission.triggers.add_triggerzone_quad(pos, pts, d.hidden, h.name, d.color)

    def _ground(self, d: _Ground) -> None:
        m, h = self.mission, d.handle
        country = m.country(h.country)
        pos = _pydcs.point(self.terrain, *d.at)
        vg = m.vehicle_group_platoon(country, h.name, d.unit_types, pos, heading=d.heading)
        for k, u in enumerate(vg.units, start=1):
            u.name = unit_name(h.name, k)
        if len(vg.units) > 1:
            getattr(vg, f"formation_{d.formation}")(d.heading, d.spacing)
        for u in vg.units:
            u.skill = d.skill
        vg.late_activation = d.late_activation
        vg.hidden = d.hidden
        self.grounds[h.name] = d

    def _own_player_airfields(self) -> None:
        """Gives every airfield a player flight starts from (parking or runway) to its coalition.

        In multiplayer DCS offers a parking slot only to the coalition that owns the airfield
        (``Scripts/UI/MultiplayerSelectRoleMap/MultiplayerSelectRoleDialog.lua``,
        ``isSlotAvailable``); at a neutral or enemy airfield the slot cannot be taken. So the
        owner is the flight's coalition, and a scenario that says otherwise is refused.
        """
        owners: dict[str, tuple[Coalition, str]] = {}
        for d in self.sc._decls:
            if not isinstance(d, _Flight) or d.handle.start is StartType.Air:
                continue
            h = d.handle
            side = self._coalition_of_country(h.country)
            seen = owners.get(h.airbase)
            if seen is not None and seen[0] is not side:
                raise ScenarioError(
                    f"player flights {seen[1]!r} ({seen[0].value}) and {h.name!r} ({side.value}) both "
                    f"start at {h.airbase}: in multiplayer DCS offers parking slots only to the "
                    "coalition that owns the airfield, so one of them could never be taken"
                )
            owners.setdefault(h.airbase, (side, h.name))
        for name, (side, flight) in owners.items():
            declared = self.sc._airbases.get(name)
            if declared is not None and declared.coalition is not None and declared.coalition is not side:
                raise ScenarioError(
                    f"player flight {flight!r} ({side.value}) starts at {name}, which the scenario gives "
                    f"to {declared.coalition.value}: in multiplayer DCS offers parking slots only to the "
                    f"coalition that owns the airfield; give {name} to {side.value} or start the flight "
                    "elsewhere"
                )
            self.terrain.airports[name].set_coalition(side.value)

    def _airbase(self, d: _Airbase) -> None:
        if d.coalition is not None:
            self.terrain.airports[d.handle.name].set_coalition(d.coalition.value)
        self.airbases.append(d)

    def _payload(self, group: Any, payload: Any, what: str) -> None:
        if payload is None:
            return
        utype = group.units[0].unit_type
        if isinstance(payload, str):
            utype.load_payloads()
            if utype.loadout_by_name(payload) is None:
                raise ScenarioError(
                    f"{what}: pydcs knows no loadout {payload!r} for {utype.id} "
                    f"(the hermetic mode has none: list the pylons, or set {_pydcs.DCS_DATA_ENV}=installed)"
                )
            group.load_loadout(payload)
            return
        for pylon in payload:
            group.load_pylon(pylon)

    def _flight(self, d: _Flight) -> None:
        m, h = self.mission, d.handle
        country = m.country(h.country)
        maintask = d.task or d.unit_type.task_default
        if maintask is None:
            raise ScenarioError(f"player flight {h.name!r}: {d.unit_type.id} has no default task; pass task=")
        if h.start is StartType.Air:
            assert d.at is not None
            pos = _pydcs.point(self.terrain, *d.at)
            fg = m.flight_group_inflight(country, h.name, d.unit_type, pos, d.altitude, maintask=maintask, group_size=h.size)
        else:
            airport = self.terrain.airports[h.airbase]
            start = {
                StartType.Cold: PydcsStartType.Cold,
                StartType.Hot: PydcsStartType.Warm,
                StartType.Runway: PydcsStartType.Runway,
            }[h.start]
            try:
                fg = m.flight_group_from_airport(
                    country, h.name, d.unit_type, airport, maintask=maintask, start_type=start, group_size=h.size
                )
            except Exception as e:  # pydcs raises NoParkingSlotError and friends
                raise ScenarioError(f"player flight {h.name!r}: {e}") from e
        for k, u in enumerate(fg.units, start=1):
            u.name = unit_name(h.name, k)
            u.skill = d.skill
        self._payload(fg, d.payload, f"player flight {h.name!r}")
        self.flights[h.name] = d

    def _template_position(self, d: _Template) -> tuple[float, float]:
        if d.at is not None:
            return d.at
        if d.airbase is not None:
            p = self.terrain.airports[d.airbase].position
            return (p.x, p.y)
        if self.airbases:
            p = self.terrain.airports[self.airbases[0].handle.name].position
            return (p.x, p.y)
        coalition = self._coalition_of_country(d.handle.country)
        bull = self.terrain.bullseye_red if coalition is Coalition.Red else self.terrain.bullseye_blue
        return (float(bull["x"]), float(bull["y"]))

    def _template(self, d: _Template) -> Any:
        h = d.handle
        country = self.scratch.country(h.country)
        pos = _pydcs.point(self.terrain, *self._template_position(d))
        g = self.scratch.flight_group_inflight(
            country, h.name, d.unit_type, pos, d.altitude, speed=d.speed, maintask=d.task, group_size=h.size
        )
        for k, u in enumerate(g.units, start=1):
            u.name = unit_name(h.name, k)
            u.skill = d.skill
        self._payload(g, d.payload, f"template {h.name!r}")
        if not any(u.pylons for u in g.units):
            hint = (
                f"pass payload=... or set {_pydcs.DCS_DATA_ENV}=installed to use the loadouts of the installed DCS"
                if _pydcs.mode() == _pydcs.HERMETIC
                else "pass payload=..."
            )
            warnings.warn(
                f"template {h.name!r} ({h.type_name}) spawns without weapons: pydcs has no default "
                f"{getattr(d.task, 'name', '')} loadout here; {hint}",
                SortieWarning,
                stacklevel=2,
            )
        return g

    # -- scanning --------------------------------------------------------------------------

    def _coalition_of_country(self, name: str) -> Coalition:
        for side, col in self.mission.coalition.items():
            if name in col.countries:
                return _COALITION_OF_SIDE.get(side, Coalition.Neutral)
        return Coalition.Neutral

    def _placed_groups(self) -> list[tuple[Coalition, Any, Any, int]]:
        """``(coalition, country, group, category)`` of every placed group, by group id."""
        out = []
        for side, col in self.mission.coalition.items():
            coalition = _COALITION_OF_SIDE.get(side, Coalition.Neutral)
            for country in col.countries.values():
                out += [(coalition, country, g, cpb.CATEGORY_GROUND_UNIT) for g in country.vehicle_group]
                out += [(coalition, country, g, cpb.CATEGORY_SHIP) for g in country.ship_group]
                out += [(coalition, country, g, cpb.CATEGORY_AIRPLANE) for g in country.plane_group]
                out += [(coalition, country, g, cpb.CATEGORY_HELICOPTER) for g in country.helicopter_group]
        out.sort(key=lambda t: t[2].id)
        return out

    def _statics(self) -> list[Any]:
        return [g for col in self.mission.coalition.values() for c in col.countries.values() for g in c.static_group]

    def _check_names(self) -> None:
        groups: set[str] = set()
        units: set[str] = set()
        for _c, _country, g, _cat in self._placed_groups():
            if g.name in groups:
                raise NameConflict(f"two groups are named {g.name!r} (DCS would destroy one of them)")
            groups.add(g.name)
            check_dcs_name("group", g.name, group=True)
            for u in g.units:
                if u.name in units:
                    raise NameConflict(f"two units are named {u.name!r}")
                units.add(u.name)
        for g in self._statics():
            for u in g.units:
                if u.name in units:
                    raise NameConflict(f"a static and a unit are both named {u.name!r}")
                units.add(u.name)

    def type_uses(self) -> list[TypeUse]:
        """Every unit type (placed groups, player flights, templates) and static object type
        of the build, once per type and place, in mission order."""
        out: list[TypeUse] = []
        seen: set[tuple[str, str]] = set()

        def add(type_name: Any, where: str, *, static: bool = False) -> None:
            key = (str(type_name), where)
            if key not in seen:
                seen.add(key)
                out.append(TypeUse(str(type_name), where, static))

        for _coalition, _country, g, category in self._placed_groups():
            flying = category in (cpb.CATEGORY_AIRPLANE, cpb.CATEGORY_HELICOPTER)
            player = flying and any(_pydcs.skill_name(u.skill) in ("Client", "Player") for u in g.units)
            where = f"{'player flight' if player else 'group'} {g.name!r}"
            for u in g.units:
                add(u.type, where)
        for d, g in self.templates:
            for u in g.units:
                add(u.type, f"template {d.handle.name!r}")
        for g in self._statics():
            for u in g.units:
                add(u.type, f"static {g.name!r}", static=True)
        return out

    def _unit_pb(self, u: Any, *, unit_id: int) -> mpb.Unit:
        x, y = float(u.position.x), float(u.position.y)
        lat, lon = _pydcs.latlon(self.terrain, x, y)
        return mpb.Unit(
            name=u.name,
            type_name=str(u.type),
            position=cpb.MapPoint(north=_clean(x), east=_clean(y)),
            alt_m=_clean(float(getattr(u, "alt", 0) or 0)),
            heading_rad=_clean(math.radians(float(u.heading or 0))),
            unit_id=unit_id,
            latlon=cpb.LatLon(lat=_clean(lat), lon=_clean(lon)),
            skill=_pydcs.skill_name(u.skill),
        )

    def manifest(self) -> mpb.Manifest:
        sc = self.sc
        m = mpb.Manifest(
            scenario=sc.name,
            schema=sc.schema,
            schema_sha256=sc.schema_sha256(),
            build_id="",
            theatre=str(self.terrain.name),
            start_time=sc.start.strftime("%Y-%m-%dT%H:%M:%SZ"),
            time_limit_s=sc.time_limit,
        )
        max_group = 0
        max_unit = 0
        for coalition, country, g, category in self._placed_groups():
            max_group = max(max_group, int(g.id))
            max_unit = max([max_unit, *(int(u.id) for u in g.units)])
            flying = category in (cpb.CATEGORY_AIRPLANE, cpb.CATEGORY_HELICOPTER)
            if flying and any(_pydcs.skill_name(u.skill) in ("Client", "Player") for u in g.units):
                m.player_flights.append(self._flight_pb(coalition, country, g))
                continue
            decl = self.grounds.get(g.name)
            pg = mpb.Group(
                ident=(decl.handle.ident or "") if decl else "",
                name=g.name,
                category=category,
                coalition=coalition.proto,
                country=country.name,
                country_id=int(country.id),
                roles=list(decl.handle.roles) if decl else [],
                group_id=int(g.id),
                late_activation=bool(getattr(g, "late_activation", False)),
            )
            pg.units.extend(self._unit_pb(u, unit_id=int(u.id)) for u in g.units)
            m.groups.append(pg)
        for g in self._statics():
            max_group = max(max_group, int(g.id))
            max_unit = max([max_unit, *(int(u.id) for u in g.units)])
        for d in sc._decls:
            if isinstance(d, _Zone) and d.handle.ident:
                h = d.handle
                z = mpb.Zone(ident=h.ident, name=h.name)
                if h.radius is not None:
                    z.circle.center.north = _clean(h.center.x)
                    z.circle.center.east = _clean(h.center.y)
                    z.circle.radius_m = float(h.radius)
                else:
                    z.polygon.points.extend(cpb.MapPoint(north=_clean(p.x), east=_clean(p.y)) for p in h.points)
                m.zones.append(z)
        for d, g in self.templates:
            m.templates.append(self._template_pb(d, g))
        for f in sc._flags.values():
            m.flags.append(mpb.Flag(ident=f.ident, name=f.name, initial=f.initial))
        for a in self.airbases:
            ap = self.terrain.airports[a.handle.name]
            m.airbases.append(
                mpb.Airbase(
                    ident=a.handle.ident,
                    name=ap.name,
                    position=cpb.MapPoint(north=_clean(ap.position.x), east=_clean(ap.position.y)),
                    airbase_id=int(ap.id),
                    coalition=Coalition.parse(ap.coalition).proto,
                )
            )
        for mn in sc._menus.values():
            m.menus.append(mpb.Menu(ident=mn.ident, label=mn.label, path=list(mn.path)))
        m.max_ids.group_id = max_group
        m.max_ids.unit_id = max_unit
        m.tools.generator = sc.generator_string()
        m.tools.pydcs = _pydcs.pydcs_version()
        m.tools.python = ".".join(platform.python_version_tuple()[:2])
        m.tools.extra["dcs_data"] = _pydcs.mode()
        return m

    def _flight_pb(self, coalition: Coalition, country: Any, g: Any) -> mpb.PlayerFlight:
        decl = self.flights.get(g.name)
        if decl is not None:
            start, airbase = decl.handle.start, decl.handle.airbase
        else:
            first = g.points[0] if g.points else None
            start = _START_OF_POINT.get(getattr(first, "type", ""), StartType.Air)
            aid = getattr(first, "airdrome_id", None)
            ap = self.terrain.airport_by_id(aid) if aid else None
            airbase = ap.name if ap is not None else ""
        f = mpb.PlayerFlight(
            name=g.name,
            type_name=str(g.units[0].type),
            size=len(g.units),
            coalition=coalition.proto,
            country=country.name,
            country_id=int(country.id),
            airbase=airbase,
            start=start.proto,
            group_id=int(g.id),
        )
        f.units.extend(self._unit_pb(u, unit_id=int(u.id)) for u in g.units)
        return f

    def _template_pb(self, d: _Template, g: Any) -> mpb.Template:
        h = d.handle
        country = self.scratch.country(h.country)
        t = mpb.Template(
            ident=h.ident,
            name=h.name,
            category=cpb.CATEGORY_HELICOPTER if h.kind == "helo" else cpb.CATEGORY_AIRPLANE,
            coalition=self._coalition_of_country(h.country).proto,
            country=h.country,
            country_id=int(country.id),
            roles=list(h.roles),
            task=str(getattr(d.task, "name", "")),
        )
        t.units.extend(self._unit_pb(u, unit_id=0) for u in g.units)
        try:
            group_json = _pydcs.template_group_json(g)
        except _pydcs.LuaShapeError as e:
            raise ScenarioError(f"template {h.name!r}: {e}") from e
        t.dcs_group_json = json.dumps(
            group_json, sort_keys=True, ensure_ascii=False, separators=(",", ":"), allow_nan=False
        )
        return t
