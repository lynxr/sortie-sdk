"""The universal call from Python: argument builders, selectors and result decoding for
:meth:`EngineClient.dcs_call <sortie_gen.engine.EngineClient.dcs_call>` and the generated
:mod:`sortie_gen.dcs_api`.

The engine checks every call against the DCS API catalog (``catalog/dcs_api.toml``) before
anything reaches DCS: the path must be listed and enabled, the arguments must fit the entry's
signature. A read takes no key; a mutating call repeats its effect unless keyed.

Values the wrappers take:

* points: ``(north, east)`` for a map point (``vec2``), ``(x, y, z)`` in runtime axes for a
  ``vec3`` (or the ``common_pb2.MapPoint`` / ``Vec3`` messages);
* coalitions: ``"red"``, ``"blue"``, ``"neutral"`` or a ``common_pb2.COALITION_*`` value;
* objects (``Target``): an Eid (``int``), a DCS name (``str``), a selector message
  (``ObjSel``, ``GroupSel``, ``AirbaseSel``, ``ControllerSel``, ``WarehouseSel``), a
  ``DcsRef``, or a :class:`DcsObject` an earlier call returned.

Results: numbers, strings and booleans as Python values, tables as ``dict`` (``list`` for
arrays 1..n), objects as :class:`DcsObject`, enum numbers as their key (``"ROAD"``), points as
tuples, coalitions as ``"red"`` / ``"blue"`` / ``"neutral"``, a table cut by a limit as
:class:`Cut`, the summary of a road path (``road_path``) as :class:`RoadPath`.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Union

from ._proto.sortie.v1 import commands_pb2, common_pb2

Point2 = Union[tuple[float, float], common_pb2.MapPoint]
Point3 = Union[tuple[float, float, float], common_pb2.Vec3]
#: A warehouse item: its name (``"weapons.missiles.AIM_9"``) or its wsType, four whole numbers
#: (``(4, 4, 7, 22)``, DCS's ``Scripts/Database/wsTypes.lua``).
Item = Union[str, tuple[int, int, int, int], list[int]]
Side = Union[str, int]
Target = Any

_SIDES = {
    "red": common_pb2.COALITION_RED,
    "blue": common_pb2.COALITION_BLUE,
    "neutral": common_pb2.COALITION_NEUTRAL,
    "neutrals": common_pb2.COALITION_NEUTRAL,
}
_SIDE_KEYS = {"RED": "red", "BLUE": "blue", "NEUTRAL": "neutral", "NEUTRALS": "neutral"}
_CLASS_NAMES = {
    commands_pb2.OBJECT_CLASS_UNIT: "unit",
    commands_pb2.OBJECT_CLASS_STATIC: "static",
    commands_pb2.OBJECT_CLASS_WEAPON: "weapon",
    commands_pb2.OBJECT_CLASS_SCENERY: "scenery",
    commands_pb2.OBJECT_CLASS_CARGO: "cargo",
    commands_pb2.OBJECT_CLASS_AIRBASE: "airbase",
    commands_pb2.OBJECT_CLASS_GROUP: "group",
    commands_pb2.OBJECT_CLASS_CONTROLLER: "controller",
    commands_pb2.OBJECT_CLASS_WAREHOUSE: "warehouse",
    commands_pb2.OBJECT_CLASS_SPOT: "spot",
    commands_pb2.OBJECT_CLASS_OTHER: "other",
}


@dataclass(frozen=True)
class Cut:
    """A table the depth or size limit cut, with its number of keys."""

    keys: int


@dataclass(frozen=True)
class DcsObject:
    """A DCS object a call returned (``ObjectRef``)."""

    object_class: str
    eid: int
    name: str
    type_name: str
    handle: int
    controller: Any = None
    warehouse: Any = None

    def ref(self) -> commands_pb2.DcsRef:
        """How a later call names this object again (receiver or argument)."""
        if self.handle:
            return commands_pb2.DcsRef(handle=self.handle)
        if self.controller is not None:
            return commands_pb2.DcsRef(controller=self.controller)
        if self.warehouse is not None:
            return commands_pb2.DcsRef(warehouse=self.warehouse)
        if self.object_class == "group":
            sel = common_pb2.GroupSel(eid=self.eid) if self.eid else common_pb2.GroupSel(name=self.name)
            return commands_pb2.DcsRef(group=sel)
        if self.object_class == "airbase":
            sel = common_pb2.AirbaseSel(eid=self.eid) if self.eid else common_pb2.AirbaseSel(name=self.name)
            return commands_pb2.DcsRef(airbase=sel)
        if self.eid:
            return commands_pb2.DcsRef(object=common_pb2.ObjSel(eid=self.eid))
        if self.object_class == "unit":
            return commands_pb2.DcsRef(object=common_pb2.ObjSel(unit_name=self.name))
        return commands_pb2.DcsRef(object=common_pb2.ObjSel(static_name=self.name))


@dataclass(frozen=True)
class RoadPath:
    """The summary of a road path the Sortie Lua layer computes inside DCS
    (``Dcs(client).land().road_path(..)``, catalog entry ``sortie.findPathOnRoads``).

    ``count``, ``length_m`` and the ends are of the whole path; ``points`` holds at most the cap
    the call asked for, both ends included, as ``(north, east)`` (none without a cap);
    ``thinned`` says it holds fewer points than the path has."""

    count: int
    length_m: float
    first: tuple[float, float] | None
    last: tuple[float, float] | None
    start_gap_m: float | None
    end_gap_m: float | None
    points: list[tuple[float, float]]
    thinned: bool


def _road_path(v: dict) -> RoadPath:
    def point(p: Any) -> tuple[float, float] | None:
        return (p.get("x"), p.get("y")) if isinstance(p, dict) else None

    raw = v.get("points")
    points = [point(p) for p in raw] if isinstance(raw, list) else []
    return RoadPath(
        count=int(v.get("count", 0)),
        length_m=float(v.get("length", 0.0)),
        first=point(v.get("first")),
        last=point(v.get("last")),
        start_gap_m=v.get("start_gap"),
        end_gap_m=v.get("end_gap"),
        points=[p for p in points if p is not None],
        thinned=bool(v.get("thinned", False)),
    )


# ----- arguments ------------------------------------------------------------------------------


def lua_value(v: Any) -> common_pb2.LuaValue:
    """Plain Python data as a ``LuaValue`` (``dict`` and ``list`` become tables)."""
    if v is None:
        return common_pb2.LuaValue(nil=common_pb2.LuaNil())
    if isinstance(v, bool):
        return common_pb2.LuaValue(boolean=v)
    if isinstance(v, (int, float)):
        return common_pb2.LuaValue(number=float(v))
    if isinstance(v, str):
        return common_pb2.LuaValue(string=v)
    if isinstance(v, dict):
        pairs = [common_pb2.LuaPair(key=lua_value(k), value=lua_value(x)) for k, x in v.items()]
        return common_pb2.LuaValue(table=common_pb2.LuaTable(pairs=pairs))
    if isinstance(v, (list, tuple)):
        pairs = [common_pb2.LuaPair(key=lua_value(i + 1), value=lua_value(x)) for i, x in enumerate(v)]
        return common_pb2.LuaValue(table=common_pb2.LuaTable(pairs=pairs))
    raise TypeError(f"{type(v).__name__} is no Lua data (None, bool, number, str, dict, list)")


def arg_nil() -> commands_pb2.DcsArg:
    return commands_pb2.DcsArg(value=lua_value(None))


def arg_number(x: float) -> commands_pb2.DcsArg:
    return commands_pb2.DcsArg(value=lua_value(float(x)))


def arg_string(s: str) -> commands_pb2.DcsArg:
    return commands_pb2.DcsArg(value=lua_value(str(s)))


def arg_boolean(b: bool) -> commands_pb2.DcsArg:
    return commands_pb2.DcsArg(value=lua_value(bool(b)))


def arg_item(item: Item) -> commands_pb2.DcsArg:
    """A warehouse item: a name as a string, a wsType as the table ``{a, b, c, d}``."""
    if isinstance(item, str):
        return arg_string(item)
    ws = list(item)
    if len(ws) != 4 or any(isinstance(n, bool) or not isinstance(n, int) or n < 0 for n in ws):
        raise ValueError(f"a wsType is four whole numbers such as (4, 4, 7, 22), not {item!r}")
    return commands_pb2.DcsArg(value=lua_value([float(n) for n in ws]))


def _has_object(v: Any) -> bool:
    if isinstance(v, DcsObject):
        return True
    if isinstance(v, dict):
        return any(_has_object(x) for x in v.values())
    if isinstance(v, (list, tuple)):
        return any(_has_object(x) for x in v)
    return False


def arg_value(v: Any) -> commands_pb2.DcsArg:
    """Data; a table holding objects (:class:`DcsObject`) is sent as a table of arguments."""
    if isinstance(v, DcsObject):
        return commands_pb2.DcsArg(object=v.ref())
    if not _has_object(v):
        return commands_pb2.DcsArg(value=lua_value(v))
    items = v.items() if isinstance(v, dict) else enumerate(v, start=1)
    fields = []
    for k, x in items:
        f = commands_pb2.DcsField(value=arg_value(x))
        if isinstance(k, int):
            f.index = k
        else:
            f.key = str(k)
        fields.append(f)
    return commands_pb2.DcsArg(table=commands_pb2.DcsTable(fields=fields))


def arg_vec2(p: Point2) -> commands_pb2.DcsArg:
    if isinstance(p, common_pb2.MapPoint):
        return commands_pb2.DcsArg(vec2=p)
    north, east = p
    return commands_pb2.DcsArg(vec2=common_pb2.MapPoint(north=float(north), east=float(east)))


def arg_vec3(p: Point3) -> commands_pb2.DcsArg:
    if isinstance(p, common_pb2.Vec3):
        return commands_pb2.DcsArg(vec3=p)
    x, y, z = p
    return commands_pb2.DcsArg(vec3=common_pb2.Vec3(x=float(x), y=float(y), z=float(z)))


def coalition(side: Side) -> int:
    if isinstance(side, str):
        try:
            return _SIDES[side.lower()]
        except KeyError:
            raise ValueError(f"unknown coalition {side!r} (red, blue or neutral)") from None
    return int(side)


def arg_coalition(side: Side) -> commands_pb2.DcsArg:
    return commands_pb2.DcsArg(coalition=coalition(side))


def arg_enum(table: str, key: str) -> commands_pb2.DcsArg:
    """A key of the DCS enum table ``table``: ``arg_enum("trigger.smokeColor", "Red")``."""
    return commands_pb2.DcsArg(enum_key=f"{table}.{key}")


def arg_player(sel: common_pb2.PlayerSel) -> commands_pb2.DcsArg:
    return commands_pb2.DcsArg(player=sel)


def arg_object(target: Target, cls: str) -> commands_pb2.DcsArg:
    return commands_pb2.DcsArg(object=receiver(target, cls))


def arg_id(target: Target, cls: str) -> commands_pb2.DcsArg:
    return commands_pb2.DcsArg(id_of=receiver(target, cls))


def arg_name(target: Target, cls: str) -> commands_pb2.DcsArg:
    return commands_pb2.DcsArg(name_of=receiver(target, cls))


def receiver(target: Target, cls: str) -> commands_pb2.DcsRef:
    """A ``DcsRef`` for an object of the catalog class ``cls`` (``"Unit"``, ``"Airbase"``, ...)."""
    if isinstance(target, commands_pb2.DcsRef):
        return target
    if isinstance(target, DcsObject):
        return target.ref()
    if isinstance(target, common_pb2.ObjSel):
        return commands_pb2.DcsRef(object=target)
    if isinstance(target, common_pb2.GroupSel):
        return commands_pb2.DcsRef(group=target)
    if isinstance(target, common_pb2.AirbaseSel):
        return commands_pb2.DcsRef(airbase=target)
    if isinstance(target, common_pb2.ControllerSel):
        return commands_pb2.DcsRef(controller=target)
    if isinstance(target, common_pb2.WarehouseSel):
        return commands_pb2.DcsRef(warehouse=target)
    if isinstance(target, bool) or not isinstance(target, (int, str)):
        raise TypeError(f"cannot name a DCS {cls} by {type(target).__name__}")
    by_eid = isinstance(target, int)
    if cls == "Spot":
        if not by_eid:
            raise TypeError("a spot is named by its handle (an int)")
        return commands_pb2.DcsRef(handle=target)
    if cls == "Group":
        g = common_pb2.GroupSel(eid=target) if by_eid else common_pb2.GroupSel(name=target)
        return commands_pb2.DcsRef(group=g)
    if cls in ("Airbase", "Warehouse"):
        a = common_pb2.AirbaseSel(eid=target) if by_eid else common_pb2.AirbaseSel(name=target)
        if cls == "Warehouse":
            return commands_pb2.DcsRef(warehouse=common_pb2.WarehouseSel(airbase=a))
        return commands_pb2.DcsRef(airbase=a)
    if cls == "Controller":
        g = common_pb2.GroupSel(eid=target) if by_eid else common_pb2.GroupSel(name=target)
        return commands_pb2.DcsRef(controller=common_pb2.ControllerSel(group=g))
    if by_eid:
        return commands_pb2.DcsRef(object=common_pb2.ObjSel(eid=target))
    if cls == "StaticObject":
        return commands_pb2.DcsRef(object=common_pb2.ObjSel(static_name=target))
    return commands_pb2.DcsRef(object=common_pb2.ObjSel(unit_name=target))


# ----- results --------------------------------------------------------------------------------


def _object(r: commands_pb2.ObjectRef) -> DcsObject:
    return DcsObject(
        object_class=_CLASS_NAMES.get(r.object_class, "other"),
        eid=r.eid,
        name=r.name,
        type_name=r.type_name,
        handle=r.handle,
        controller=r.controller if r.HasField("controller") else None,
        warehouse=r.warehouse if r.HasField("warehouse") else None,
    )


def value(v: commands_pb2.DcsValue, refs: list[DcsObject]) -> Any:
    """One ``DcsValue`` as Python data."""
    kind = v.WhichOneof("kind")
    if kind is None or kind == "nil":
        return None
    if kind == "boolean":
        return v.boolean
    if kind == "number":
        return v.number
    if kind == "string":
        return v.string
    if kind == "enum_key":
        return v.enum_key
    if kind == "ref":
        return refs[v.ref] if v.ref < len(refs) else None
    if kind == "cut":
        return Cut(v.cut)
    pairs = [(value(p.key, refs), value(p.value, refs)) for p in v.table.pairs]
    keys = [k for k, _ in pairs]
    if pairs and keys == [float(i) for i in range(1, len(pairs) + 1)]:
        return [x for _, x in pairs]
    return {(int(k) if isinstance(k, float) and k.is_integer() else k): x for k, x in pairs}


def _shape(v: Any, notation: str) -> Any:
    ty = notation[:-1] if notation.endswith("?") else notation
    if v is None:
        return None
    if ty == "side_number":
        return _SIDE_KEYS.get(v, v) if isinstance(v, str) else v
    if ty == "vec3" and isinstance(v, dict):
        return (v.get("x"), v.get("y"), v.get("z"))
    if ty == "vec2" and isinstance(v, dict):
        return (v.get("x"), v.get("y"))
    if ty.startswith("list:"):
        if isinstance(v, dict) and not v:
            return []
        if isinstance(v, list):
            return [_shape(x, ty[5:]) for x in v]
    if ty.startswith("handle:") and isinstance(v, DcsObject):
        return v.handle
    if ty == "road_path" and isinstance(v, dict):
        return _road_path(v)
    return v


def decode_return(ret: commands_pb2.DcsReturn, returns: list[str]) -> Any:
    """The values of a ``DcsReturn`` shaped by the entry's return types: nothing -> ``None``,
    one value -> that value, more -> a tuple."""
    refs = [_object(r) for r in ret.refs]
    values = [value(v, refs) for v in ret.values]
    shaped = [_shape(values[i] if i < len(values) else None, r) for i, r in enumerate(returns)]
    if not returns:
        return None
    if len(returns) == 1:
        return shaped[0]
    return tuple(shaped)
