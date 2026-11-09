"""The only module that talks to pydcs internals.

pydcs 1.0.0 is used unchanged. This adapter

* silences the ``SyntaxWarning`` pydcs emits the first time its weapon tables are compiled;
* makes pydcs *hermetic* by default: it does not look for an installed DCS (registry, Saved
  Games) for payloads and liveries, so the same scenario produces the same files on every
  machine. ``SORTIE_GEN_DCS_DATA=installed`` restores pydcs's own lookup (default loadouts and
  liveries from the local DCS installation, output then depends on that installation);
* makes tail numbers deterministic (pydcs 1.0.0 pops them from a ``set`` of strings, whose
  order changes with ``PYTHONHASHSEED``; here the lowest free number is taken);
* rounds projection results (lat/lon <-> map point, done by pyproj) to a fixed precision, so
  Windows and Linux write the same bytes;
* converts pydcs group dicts (Lua-shaped, integer keys) into JSON for ``dcs_group_json``;
* wraps the few pydcs calls whose arguments are easy to get wrong.
"""

from __future__ import annotations

import enum
import math
import os
import warnings
from pathlib import Path
from typing import Any

warnings.filterwarnings("ignore", message=r"invalid escape sequence", category=SyntaxWarning)

import dcs  # noqa: E402  (after the warning filter on purpose)
import dcs.country  # noqa: E402
import dcs.mapping  # noqa: E402
import dcs.payloads  # noqa: E402
import dcs.unittype  # noqa: E402
from dcs.liveries.liverycache import LiveryCache  # noqa: E402

DCS_DATA_ENV = "SORTIE_GEN_DCS_DATA"
HERMETIC = "hermetic"
INSTALLED = "installed"

_mode: str | None = None


def configure(mode: str | None = None) -> str:
    """Selects how pydcs finds payloads and liveries (see the module docs); returns the mode.

    Must run before the first aircraft is created; ``sortie_gen`` calls it on import.
    """
    global _mode
    mode = (mode or os.environ.get(DCS_DATA_ENV) or HERMETIC).strip().lower()
    if mode not in (HERMETIC, INSTALLED):
        raise ValueError(f"{DCS_DATA_ENV} must be {HERMETIC!r} or {INSTALLED!r}, not {mode!r}")
    if mode == HERMETIC:
        dirs = dcs.payloads.PayloadDirectories
        dirs._dcs = []
        dirs._mod = []
        dirs._user = Path(os.devnull) / "sortie-gen-hermetic"
        LiveryCache._cache = {}
    _mode = mode
    return mode


def _lowest_free_onboard_num(self: Any) -> str:
    """Drop-in for ``Country.next_onboard_num``: the lowest free tail number, 010..998."""
    for n in range(10, 999):
        tail = f"{n:03}"
        if tail not in self._tail_numbers:
            self.reserve_onboard_num(tail)
            return tail
    raise ValueError(f"{self.name}: all tail numbers 010..998 are taken")


dcs.country.Country.next_onboard_num = _lowest_free_onboard_num


def mode() -> str:
    return _mode or configure()


def pydcs_version() -> str:
    from importlib.metadata import PackageNotFoundError, version

    try:
        return version("pydcs")
    except PackageNotFoundError:  # pragma: no cover - only in odd source checkouts
        return "unknown"


# ----- geometry --------------------------------------------------------------------------------


#: Decimal places kept from projection results. pydcs projects with pyproj, whose PROJ library
#: uses the platform's C maths functions, so Windows and Linux can differ in the last bits of a
#: projected coordinate. Rounding here makes the manifest, the lock, the build id and the
#: ``.miz`` entries match across platforms. An n-ulp difference survives only if it straddles a
#: rounding midpoint (chance about n * ulp / quantum; geometry computed outside pydcs, e.g. in
#: the scenario's own maths, is not covered).
LATLON_DIGITS = 9  # degrees: 1e-9 deg is about 0.1 mm
POINT_DIGITS = 3  # metres: with 1e-9 m the chance would be about 6 % near 500 km


def _quantize(x: float, digits: int) -> float:
    return round(float(x), digits) + 0.0  # + 0.0: never -0.0


def point(terrain: Any, north: float, east: float) -> dcs.mapping.Point:
    return dcs.mapping.Point(float(north), float(east), terrain)


def point_from_latlon(terrain: Any, lat: float, lon: float) -> dcs.mapping.Point:
    """The map point of a WGS84 position, rounded to :data:`POINT_DIGITS` (a projection)."""
    p = dcs.mapping.Point.from_latlng(dcs.mapping.LatLng(float(lat), float(lon)), terrain)
    return point(terrain, _quantize(p.x, POINT_DIGITS), _quantize(p.y, POINT_DIGITS))


def latlon(terrain: Any, north: float, east: float) -> tuple[float, float]:
    """WGS84 latitude/longitude of a map point, rounded to :data:`LATLON_DIGITS`."""
    ll = point(terrain, north, east).latlng()
    return (_quantize(ll.lat, LATLON_DIGITS), _quantize(ll.lng, LATLON_DIGITS))


# ----- unit types -----------------------------------------------------------------------------


def is_vehicle_type(t: Any) -> bool:
    return isinstance(t, type) and issubclass(t, dcs.unittype.VehicleType)


def is_flying_type(t: Any) -> bool:
    return isinstance(t, type) and issubclass(t, dcs.unittype.FlyingType)


def type_id(t: Any) -> str:
    return str(t.id)


# ----- Lua-shaped dicts -> JSON ---------------------------------------------------------------

#: The largest integer key a sparse table may have before it is refused (JSON arrays with
#: ``null`` holes stand for sparse Lua arrays such as pylon tables).
MAX_SPARSE_KEY = 1024


class LuaShapeError(ValueError):
    """A pydcs value has no faithful JSON form."""


def to_json_value(value: Any, path: str = "") -> Any:
    """Converts a pydcs ``dict()`` value into JSON-compatible data.

    * dicts whose keys are all integers become arrays (``null`` in the holes, which the engine
      turns back into a sparse Lua table: pylons ``{1, 3}`` stay pylons 1 and 3);
    * dicts with string keys stay objects;
    * dicts that mix integer and string keys cannot be written as JSON and raise
      :class:`LuaShapeError` (callers drop such keys explicitly, see ``group_json``);
    * pydcs enum-like classes (``Targets.All.Air``) become their DCS id, enums their value;
    * ``-0.0`` becomes ``0.0`` and non-finite floats are refused.
    """
    if isinstance(value, dict):
        keys = list(value.keys())
        if keys and all(isinstance(k, int) and not isinstance(k, bool) for k in keys):
            top = max(keys)
            if min(keys) < 1 or top > MAX_SPARSE_KEY:
                raise LuaShapeError(f"{path or '<root>'}: integer keys outside 1..{MAX_SPARSE_KEY}")
            out: list[Any] = [None] * top
            for k in keys:
                out[k - 1] = to_json_value(value[k], f"{path}[{k}]")
            return out
        if any(not isinstance(k, str) for k in keys):
            raise LuaShapeError(f"{path or '<root>'}: a table mixes integer and string keys")
        return {k: to_json_value(value[k], f"{path}.{k}" if path else k) for k in keys}
    if isinstance(value, (list, tuple)):
        return [to_json_value(v, f"{path}[{i + 1}]") for i, v in enumerate(value)]
    if isinstance(value, bool) or value is None or isinstance(value, str):
        return value
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        if not math.isfinite(value):
            raise LuaShapeError(f"{path}: non-finite number {value!r}")
        return value + 0.0
    if isinstance(value, enum.Enum):
        return to_json_value(value.value, path)
    if isinstance(value, type) and hasattr(value, "id"):
        return str(value.id)
    raise LuaShapeError(f"{path}: cannot convert {type(value).__name__} to JSON")


#: Keys removed from template group tables: runtime spawns never carry ids (DCS assigns
#: them), and callsigns mix integer and string keys (DCS picks one at spawn).
_TEMPLATE_DROP_GROUP = ("groupId",)
_TEMPLATE_DROP_UNIT = ("unitId", "callsign", "parking", "parking_id")


def template_group_json(group: Any) -> dict[str, Any]:
    """The JSON form of a pydcs flying/vehicle group used as a spawn template."""
    d = dict(group.dict())
    for k in _TEMPLATE_DROP_GROUP:
        d.pop(k, None)
    units = d.get("units", {})
    clean_units = {}
    for idx, u in units.items():
        u = dict(u)
        for k in _TEMPLATE_DROP_UNIT:
            u.pop(k, None)
        clean_units[idx] = u
    d["units"] = clean_units
    out = to_json_value(d)
    if not isinstance(out, dict):
        raise LuaShapeError("a group table must be an object")
    return out


def skill_name(skill: Any) -> str:
    if isinstance(skill, enum.Enum):
        return str(skill.value)
    return str(skill) if skill is not None else ""
