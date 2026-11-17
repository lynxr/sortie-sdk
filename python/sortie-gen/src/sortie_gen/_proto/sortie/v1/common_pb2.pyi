from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class Coalition(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    COALITION_UNSPECIFIED: _ClassVar[Coalition]
    COALITION_NEUTRAL: _ClassVar[Coalition]
    COALITION_RED: _ClassVar[Coalition]
    COALITION_BLUE: _ClassVar[Coalition]

class Category(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CATEGORY_UNSPECIFIED: _ClassVar[Category]
    CATEGORY_AIRPLANE: _ClassVar[Category]
    CATEGORY_HELICOPTER: _ClassVar[Category]
    CATEGORY_GROUND_UNIT: _ClassVar[Category]
    CATEGORY_SHIP: _ClassVar[Category]
    CATEGORY_STRUCTURE: _ClassVar[Category]
    CATEGORY_STATIC: _ClassVar[Category]
    CATEGORY_WEAPON: _ClassVar[Category]
    CATEGORY_SCENERY: _ClassVar[Category]
    CATEGORY_BASE: _ClassVar[Category]
    CATEGORY_CARGO: _ClassVar[Category]

class Env(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ENV_UNSPECIFIED: _ClassVar[Env]
    ENV_HOOKS: _ClassVar[Env]
    ENV_MISSION: _ClassVar[Env]

class Outcome(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OUTCOME_UNSPECIFIED: _ClassVar[Outcome]
    OUTCOME_BLUE_WIN: _ClassVar[Outcome]
    OUTCOME_RED_WIN: _ClassVar[Outcome]
    OUTCOME_DRAW: _ClassVar[Outcome]
    OUTCOME_ABORTED: _ClassVar[Outcome]
    OUTCOME_TIME_LIMIT: _ClassVar[Outcome]

class StartType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    START_TYPE_UNSPECIFIED: _ClassVar[StartType]
    START_TYPE_RUNWAY: _ClassVar[StartType]
    START_TYPE_HOT: _ClassVar[StartType]
    START_TYPE_COLD: _ClassVar[StartType]
    START_TYPE_AIR: _ClassVar[StartType]

class AlarmState(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ALARM_STATE_UNSPECIFIED: _ClassVar[AlarmState]
    ALARM_STATE_AUTO: _ClassVar[AlarmState]
    ALARM_STATE_GREEN: _ClassVar[AlarmState]
    ALARM_STATE_RED: _ClassVar[AlarmState]

class Roe(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ROE_UNSPECIFIED: _ClassVar[Roe]
    ROE_WEAPON_FREE: _ClassVar[Roe]
    ROE_OPEN_FIRE_WEAPON_FREE: _ClassVar[Roe]
    ROE_OPEN_FIRE: _ClassVar[Roe]
    ROE_RETURN_FIRE: _ClassVar[Roe]
    ROE_WEAPON_HOLD: _ClassVar[Roe]

class SmokeColor(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SMOKE_COLOR_UNSPECIFIED: _ClassVar[SmokeColor]
    SMOKE_COLOR_GREEN: _ClassVar[SmokeColor]
    SMOKE_COLOR_RED: _ClassVar[SmokeColor]
    SMOKE_COLOR_WHITE: _ClassVar[SmokeColor]
    SMOKE_COLOR_ORANGE: _ClassVar[SmokeColor]
    SMOKE_COLOR_BLUE: _ClassVar[SmokeColor]

class ErrorCode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ERROR_CODE_UNSPECIFIED: _ClassVar[ErrorCode]
    ERROR_CODE_GONE: _ClassVar[ErrorCode]
    ERROR_CODE_TIMEOUT: _ClassVar[ErrorCode]
    ERROR_CODE_LUA_ERROR: _ClassVar[ErrorCode]
    ERROR_CODE_BRIDGE_FAULT: _ClassVar[ErrorCode]
    ERROR_CODE_REJECTED: _ClassVar[ErrorCode]
    ERROR_CODE_SESSION_ENDED: _ClassVar[ErrorCode]
    ERROR_CODE_INVALID: _ClassVar[ErrorCode]
    ERROR_CODE_UNAVAILABLE: _ClassVar[ErrorCode]
    ERROR_CODE_NAME_CONFLICT: _ClassVar[ErrorCode]
    ERROR_CODE_PAUSED: _ClassVar[ErrorCode]
    ERROR_CODE_QUARANTINED: _ClassVar[ErrorCode]
    ERROR_CODE_CONFLICT: _ClassVar[ErrorCode]

class Modulation(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODULATION_UNSPECIFIED: _ClassVar[Modulation]
    MODULATION_AM: _ClassVar[Modulation]
    MODULATION_FM: _ClassVar[Modulation]

class NameConflict(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NAME_CONFLICT_UNSPECIFIED: _ClassVar[NameConflict]
    NAME_CONFLICT_FAIL: _ClassVar[NameConflict]
    NAME_CONFLICT_REPLACE: _ClassVar[NameConflict]
    NAME_CONFLICT_KEEP: _ClassVar[NameConflict]

class CallClass(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CALL_CLASS_UNSPECIFIED: _ClassVar[CallClass]
    CALL_CLASS_READ: _ClassVar[CallClass]
    CALL_CLASS_MUTATE: _ClassVar[CallClass]
    CALL_CLASS_HEAVY: _ClassVar[CallClass]
    CALL_CLASS_ADMIN: _ClassVar[CallClass]

class CapabilityState(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    CAPABILITY_STATE_UNSPECIFIED: _ClassVar[CapabilityState]
    CAPABILITY_STATE_AVAILABLE: _ClassVar[CapabilityState]
    CAPABILITY_STATE_NOT_IMPLEMENTED: _ClassVar[CapabilityState]
    CAPABILITY_STATE_MISSING_IN_DCS: _ClassVar[CapabilityState]
    CAPABILITY_STATE_DISABLED: _ClassVar[CapabilityState]
    CAPABILITY_STATE_QUARANTINED: _ClassVar[CapabilityState]

class LiveStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LIVE_STATUS_UNSPECIFIED: _ClassVar[LiveStatus]
    LIVE_STATUS_UNVERIFIED: _ClassVar[LiveStatus]
    LIVE_STATUS_VERIFIED: _ClassVar[LiveStatus]
    LIVE_STATUS_FAILED: _ClassVar[LiveStatus]
COALITION_UNSPECIFIED: Coalition
COALITION_NEUTRAL: Coalition
COALITION_RED: Coalition
COALITION_BLUE: Coalition
CATEGORY_UNSPECIFIED: Category
CATEGORY_AIRPLANE: Category
CATEGORY_HELICOPTER: Category
CATEGORY_GROUND_UNIT: Category
CATEGORY_SHIP: Category
CATEGORY_STRUCTURE: Category
CATEGORY_STATIC: Category
CATEGORY_WEAPON: Category
CATEGORY_SCENERY: Category
CATEGORY_BASE: Category
CATEGORY_CARGO: Category
ENV_UNSPECIFIED: Env
ENV_HOOKS: Env
ENV_MISSION: Env
OUTCOME_UNSPECIFIED: Outcome
OUTCOME_BLUE_WIN: Outcome
OUTCOME_RED_WIN: Outcome
OUTCOME_DRAW: Outcome
OUTCOME_ABORTED: Outcome
OUTCOME_TIME_LIMIT: Outcome
START_TYPE_UNSPECIFIED: StartType
START_TYPE_RUNWAY: StartType
START_TYPE_HOT: StartType
START_TYPE_COLD: StartType
START_TYPE_AIR: StartType
ALARM_STATE_UNSPECIFIED: AlarmState
ALARM_STATE_AUTO: AlarmState
ALARM_STATE_GREEN: AlarmState
ALARM_STATE_RED: AlarmState
ROE_UNSPECIFIED: Roe
ROE_WEAPON_FREE: Roe
ROE_OPEN_FIRE_WEAPON_FREE: Roe
ROE_OPEN_FIRE: Roe
ROE_RETURN_FIRE: Roe
ROE_WEAPON_HOLD: Roe
SMOKE_COLOR_UNSPECIFIED: SmokeColor
SMOKE_COLOR_GREEN: SmokeColor
SMOKE_COLOR_RED: SmokeColor
SMOKE_COLOR_WHITE: SmokeColor
SMOKE_COLOR_ORANGE: SmokeColor
SMOKE_COLOR_BLUE: SmokeColor
ERROR_CODE_UNSPECIFIED: ErrorCode
ERROR_CODE_GONE: ErrorCode
ERROR_CODE_TIMEOUT: ErrorCode
ERROR_CODE_LUA_ERROR: ErrorCode
ERROR_CODE_BRIDGE_FAULT: ErrorCode
ERROR_CODE_REJECTED: ErrorCode
ERROR_CODE_SESSION_ENDED: ErrorCode
ERROR_CODE_INVALID: ErrorCode
ERROR_CODE_UNAVAILABLE: ErrorCode
ERROR_CODE_NAME_CONFLICT: ErrorCode
ERROR_CODE_PAUSED: ErrorCode
ERROR_CODE_QUARANTINED: ErrorCode
ERROR_CODE_CONFLICT: ErrorCode
MODULATION_UNSPECIFIED: Modulation
MODULATION_AM: Modulation
MODULATION_FM: Modulation
NAME_CONFLICT_UNSPECIFIED: NameConflict
NAME_CONFLICT_FAIL: NameConflict
NAME_CONFLICT_REPLACE: NameConflict
NAME_CONFLICT_KEEP: NameConflict
CALL_CLASS_UNSPECIFIED: CallClass
CALL_CLASS_READ: CallClass
CALL_CLASS_MUTATE: CallClass
CALL_CLASS_HEAVY: CallClass
CALL_CLASS_ADMIN: CallClass
CAPABILITY_STATE_UNSPECIFIED: CapabilityState
CAPABILITY_STATE_AVAILABLE: CapabilityState
CAPABILITY_STATE_NOT_IMPLEMENTED: CapabilityState
CAPABILITY_STATE_MISSING_IN_DCS: CapabilityState
CAPABILITY_STATE_DISABLED: CapabilityState
CAPABILITY_STATE_QUARANTINED: CapabilityState
LIVE_STATUS_UNSPECIFIED: LiveStatus
LIVE_STATUS_UNVERIFIED: LiveStatus
LIVE_STATUS_VERIFIED: LiveStatus
LIVE_STATUS_FAILED: LiveStatus

class Vec3(_message.Message):
    __slots__ = ("x", "y", "z")
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    x: float
    y: float
    z: float
    def __init__(self, x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ...) -> None: ...

class MapPoint(_message.Message):
    __slots__ = ("north", "east")
    NORTH_FIELD_NUMBER: _ClassVar[int]
    EAST_FIELD_NUMBER: _ClassVar[int]
    north: float
    east: float
    def __init__(self, north: _Optional[float] = ..., east: _Optional[float] = ...) -> None: ...

class LatLon(_message.Message):
    __slots__ = ("lat", "lon")
    LAT_FIELD_NUMBER: _ClassVar[int]
    LON_FIELD_NUMBER: _ClassVar[int]
    lat: float
    lon: float
    def __init__(self, lat: _Optional[float] = ..., lon: _Optional[float] = ...) -> None: ...

class CommandError(_message.Message):
    __slots__ = ("code", "message", "lua_traceback", "capability")
    CODE_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    LUA_TRACEBACK_FIELD_NUMBER: _ClassVar[int]
    CAPABILITY_FIELD_NUMBER: _ClassVar[int]
    code: ErrorCode
    message: str
    lua_traceback: str
    capability: str
    def __init__(self, code: _Optional[_Union[ErrorCode, str]] = ..., message: _Optional[str] = ..., lua_traceback: _Optional[str] = ..., capability: _Optional[str] = ...) -> None: ...

class LuaValue(_message.Message):
    __slots__ = ("nil", "boolean", "number", "string", "table")
    NIL_FIELD_NUMBER: _ClassVar[int]
    BOOLEAN_FIELD_NUMBER: _ClassVar[int]
    NUMBER_FIELD_NUMBER: _ClassVar[int]
    STRING_FIELD_NUMBER: _ClassVar[int]
    TABLE_FIELD_NUMBER: _ClassVar[int]
    nil: LuaNil
    boolean: bool
    number: float
    string: str
    table: LuaTable
    def __init__(self, nil: _Optional[_Union[LuaNil, _Mapping]] = ..., boolean: _Optional[bool] = ..., number: _Optional[float] = ..., string: _Optional[str] = ..., table: _Optional[_Union[LuaTable, _Mapping]] = ...) -> None: ...

class LuaNil(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class LuaTable(_message.Message):
    __slots__ = ("pairs",)
    PAIRS_FIELD_NUMBER: _ClassVar[int]
    pairs: _containers.RepeatedCompositeFieldContainer[LuaPair]
    def __init__(self, pairs: _Optional[_Iterable[_Union[LuaPair, _Mapping]]] = ...) -> None: ...

class LuaPair(_message.Message):
    __slots__ = ("key", "value")
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    key: LuaValue
    value: LuaValue
    def __init__(self, key: _Optional[_Union[LuaValue, _Mapping]] = ..., value: _Optional[_Union[LuaValue, _Mapping]] = ...) -> None: ...

class GroupSel(_message.Message):
    __slots__ = ("eid", "name", "ident")
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    IDENT_FIELD_NUMBER: _ClassVar[int]
    eid: int
    name: str
    ident: str
    def __init__(self, eid: _Optional[int] = ..., name: _Optional[str] = ..., ident: _Optional[str] = ...) -> None: ...

class UnitSel(_message.Message):
    __slots__ = ("eid", "name")
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    eid: int
    name: str
    def __init__(self, eid: _Optional[int] = ..., name: _Optional[str] = ...) -> None: ...

class ObjSel(_message.Message):
    __slots__ = ("eid", "unit_name", "static_name", "scenery_name")
    EID_FIELD_NUMBER: _ClassVar[int]
    UNIT_NAME_FIELD_NUMBER: _ClassVar[int]
    STATIC_NAME_FIELD_NUMBER: _ClassVar[int]
    SCENERY_NAME_FIELD_NUMBER: _ClassVar[int]
    eid: int
    unit_name: str
    static_name: str
    scenery_name: str
    def __init__(self, eid: _Optional[int] = ..., unit_name: _Optional[str] = ..., static_name: _Optional[str] = ..., scenery_name: _Optional[str] = ...) -> None: ...

class AirbaseSel(_message.Message):
    __slots__ = ("eid", "name", "ident")
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    IDENT_FIELD_NUMBER: _ClassVar[int]
    eid: int
    name: str
    ident: str
    def __init__(self, eid: _Optional[int] = ..., name: _Optional[str] = ..., ident: _Optional[str] = ...) -> None: ...

class PlayerSel(_message.Message):
    __slots__ = ("ucid", "player_id", "name")
    UCID_FIELD_NUMBER: _ClassVar[int]
    PLAYER_ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ucid: str
    player_id: int
    name: str
    def __init__(self, ucid: _Optional[str] = ..., player_id: _Optional[int] = ..., name: _Optional[str] = ...) -> None: ...

class WarehouseSel(_message.Message):
    __slots__ = ("airbase", "cargo", "dcs_name")
    AIRBASE_FIELD_NUMBER: _ClassVar[int]
    CARGO_FIELD_NUMBER: _ClassVar[int]
    DCS_NAME_FIELD_NUMBER: _ClassVar[int]
    airbase: AirbaseSel
    cargo: ObjSel
    dcs_name: str
    def __init__(self, airbase: _Optional[_Union[AirbaseSel, _Mapping]] = ..., cargo: _Optional[_Union[ObjSel, _Mapping]] = ..., dcs_name: _Optional[str] = ...) -> None: ...

class ZoneSel(_message.Message):
    __slots__ = ("ident", "name")
    IDENT_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ident: str
    name: str
    def __init__(self, ident: _Optional[str] = ..., name: _Optional[str] = ...) -> None: ...

class ControllerSel(_message.Message):
    __slots__ = ("group", "unit")
    GROUP_FIELD_NUMBER: _ClassVar[int]
    UNIT_FIELD_NUMBER: _ClassVar[int]
    group: GroupSel
    unit: UnitSel
    def __init__(self, group: _Optional[_Union[GroupSel, _Mapping]] = ..., unit: _Optional[_Union[UnitSel, _Mapping]] = ...) -> None: ...

class Audience(_message.Message):
    __slots__ = ("all", "coalition", "country", "group", "unit", "player")
    ALL_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    COUNTRY_FIELD_NUMBER: _ClassVar[int]
    GROUP_FIELD_NUMBER: _ClassVar[int]
    UNIT_FIELD_NUMBER: _ClassVar[int]
    PLAYER_FIELD_NUMBER: _ClassVar[int]
    all: bool
    coalition: Coalition
    country: str
    group: GroupSel
    unit: UnitSel
    player: PlayerSel
    def __init__(self, all: _Optional[bool] = ..., coalition: _Optional[_Union[Coalition, str]] = ..., country: _Optional[str] = ..., group: _Optional[_Union[GroupSel, _Mapping]] = ..., unit: _Optional[_Union[UnitSel, _Mapping]] = ..., player: _Optional[_Union[PlayerSel, _Mapping]] = ...) -> None: ...

class Anchor(_message.Message):
    __slots__ = ("point", "ground", "object", "zone", "height_offset_m")
    POINT_FIELD_NUMBER: _ClassVar[int]
    GROUND_FIELD_NUMBER: _ClassVar[int]
    OBJECT_FIELD_NUMBER: _ClassVar[int]
    ZONE_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_OFFSET_M_FIELD_NUMBER: _ClassVar[int]
    point: Vec3
    ground: MapPoint
    object: ObjSel
    zone: ZoneSel
    height_offset_m: float
    def __init__(self, point: _Optional[_Union[Vec3, _Mapping]] = ..., ground: _Optional[_Union[MapPoint, _Mapping]] = ..., object: _Optional[_Union[ObjSel, _Mapping]] = ..., zone: _Optional[_Union[ZoneSel, _Mapping]] = ..., height_offset_m: _Optional[float] = ...) -> None: ...

class MapCircle(_message.Message):
    __slots__ = ("center", "radius_m")
    CENTER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_M_FIELD_NUMBER: _ClassVar[int]
    center: MapPoint
    radius_m: float
    def __init__(self, center: _Optional[_Union[MapPoint, _Mapping]] = ..., radius_m: _Optional[float] = ...) -> None: ...

class MapPolygon(_message.Message):
    __slots__ = ("points",)
    POINTS_FIELD_NUMBER: _ClassVar[int]
    points: _containers.RepeatedCompositeFieldContainer[MapPoint]
    def __init__(self, points: _Optional[_Iterable[_Union[MapPoint, _Mapping]]] = ...) -> None: ...

class AroundObject(_message.Message):
    __slots__ = ("object", "radius_m")
    OBJECT_FIELD_NUMBER: _ClassVar[int]
    RADIUS_M_FIELD_NUMBER: _ClassVar[int]
    object: ObjSel
    radius_m: float
    def __init__(self, object: _Optional[_Union[ObjSel, _Mapping]] = ..., radius_m: _Optional[float] = ...) -> None: ...

class MapArea(_message.Message):
    __slots__ = ("zone", "circle", "polygon", "around")
    ZONE_FIELD_NUMBER: _ClassVar[int]
    CIRCLE_FIELD_NUMBER: _ClassVar[int]
    POLYGON_FIELD_NUMBER: _ClassVar[int]
    AROUND_FIELD_NUMBER: _ClassVar[int]
    zone: ZoneSel
    circle: MapCircle
    polygon: MapPolygon
    around: AroundObject
    def __init__(self, zone: _Optional[_Union[ZoneSel, _Mapping]] = ..., circle: _Optional[_Union[MapCircle, _Mapping]] = ..., polygon: _Optional[_Union[MapPolygon, _Mapping]] = ..., around: _Optional[_Union[AroundObject, _Mapping]] = ...) -> None: ...

class Volume(_message.Message):
    __slots__ = ("sphere", "axis_box", "segment", "pyramid", "area")
    SPHERE_FIELD_NUMBER: _ClassVar[int]
    AXIS_BOX_FIELD_NUMBER: _ClassVar[int]
    SEGMENT_FIELD_NUMBER: _ClassVar[int]
    PYRAMID_FIELD_NUMBER: _ClassVar[int]
    AREA_FIELD_NUMBER: _ClassVar[int]
    sphere: Sphere
    axis_box: AxisBox
    segment: Segment
    pyramid: Pyramid
    area: AreaVolume
    def __init__(self, sphere: _Optional[_Union[Sphere, _Mapping]] = ..., axis_box: _Optional[_Union[AxisBox, _Mapping]] = ..., segment: _Optional[_Union[Segment, _Mapping]] = ..., pyramid: _Optional[_Union[Pyramid, _Mapping]] = ..., area: _Optional[_Union[AreaVolume, _Mapping]] = ...) -> None: ...

class Sphere(_message.Message):
    __slots__ = ("center", "radius_m", "center_on_ground")
    CENTER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_M_FIELD_NUMBER: _ClassVar[int]
    CENTER_ON_GROUND_FIELD_NUMBER: _ClassVar[int]
    center: Vec3
    radius_m: float
    center_on_ground: bool
    def __init__(self, center: _Optional[_Union[Vec3, _Mapping]] = ..., radius_m: _Optional[float] = ..., center_on_ground: _Optional[bool] = ...) -> None: ...

class AxisBox(_message.Message):
    __slots__ = ("min", "max")
    MIN_FIELD_NUMBER: _ClassVar[int]
    MAX_FIELD_NUMBER: _ClassVar[int]
    min: Vec3
    max: Vec3
    def __init__(self, min: _Optional[_Union[Vec3, _Mapping]] = ..., max: _Optional[_Union[Vec3, _Mapping]] = ...) -> None: ...

class Segment(_message.Message):
    __slots__ = ("start", "end")
    START_FIELD_NUMBER: _ClassVar[int]
    END_FIELD_NUMBER: _ClassVar[int]
    start: Vec3
    end: Vec3
    def __init__(self, start: _Optional[_Union[Vec3, _Mapping]] = ..., end: _Optional[_Union[Vec3, _Mapping]] = ...) -> None: ...

class Pyramid(_message.Message):
    __slots__ = ("apex", "heading_rad", "pitch_rad", "length_m", "half_angle_hor_rad", "half_angle_ver_rad")
    APEX_FIELD_NUMBER: _ClassVar[int]
    HEADING_RAD_FIELD_NUMBER: _ClassVar[int]
    PITCH_RAD_FIELD_NUMBER: _ClassVar[int]
    LENGTH_M_FIELD_NUMBER: _ClassVar[int]
    HALF_ANGLE_HOR_RAD_FIELD_NUMBER: _ClassVar[int]
    HALF_ANGLE_VER_RAD_FIELD_NUMBER: _ClassVar[int]
    apex: Vec3
    heading_rad: float
    pitch_rad: float
    length_m: float
    half_angle_hor_rad: float
    half_angle_ver_rad: float
    def __init__(self, apex: _Optional[_Union[Vec3, _Mapping]] = ..., heading_rad: _Optional[float] = ..., pitch_rad: _Optional[float] = ..., length_m: _Optional[float] = ..., half_angle_hor_rad: _Optional[float] = ..., half_angle_ver_rad: _Optional[float] = ...) -> None: ...

class AreaVolume(_message.Message):
    __slots__ = ("area", "floor_m", "ceiling_m")
    AREA_FIELD_NUMBER: _ClassVar[int]
    FLOOR_M_FIELD_NUMBER: _ClassVar[int]
    CEILING_M_FIELD_NUMBER: _ClassVar[int]
    area: MapArea
    floor_m: float
    ceiling_m: float
    def __init__(self, area: _Optional[_Union[MapArea, _Mapping]] = ..., floor_m: _Optional[float] = ..., ceiling_m: _Optional[float] = ...) -> None: ...

class Rgba(_message.Message):
    __slots__ = ("r", "g", "b", "a")
    R_FIELD_NUMBER: _ClassVar[int]
    G_FIELD_NUMBER: _ClassVar[int]
    B_FIELD_NUMBER: _ClassVar[int]
    A_FIELD_NUMBER: _ClassVar[int]
    r: float
    g: float
    b: float
    a: float
    def __init__(self, r: _Optional[float] = ..., g: _Optional[float] = ..., b: _Optional[float] = ..., a: _Optional[float] = ...) -> None: ...

class CallSignature(_message.Message):
    __slots__ = ("env", "call_class", "kind", "receiver", "args", "returns", "guards")
    ENV_FIELD_NUMBER: _ClassVar[int]
    CALL_CLASS_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    RECEIVER_FIELD_NUMBER: _ClassVar[int]
    ARGS_FIELD_NUMBER: _ClassVar[int]
    RETURNS_FIELD_NUMBER: _ClassVar[int]
    GUARDS_FIELD_NUMBER: _ClassVar[int]
    env: Env
    call_class: CallClass
    kind: str
    receiver: str
    args: _containers.RepeatedScalarFieldContainer[str]
    returns: _containers.RepeatedScalarFieldContainer[str]
    guards: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, env: _Optional[_Union[Env, str]] = ..., call_class: _Optional[_Union[CallClass, str]] = ..., kind: _Optional[str] = ..., receiver: _Optional[str] = ..., args: _Optional[_Iterable[str]] = ..., returns: _Optional[_Iterable[str]] = ..., guards: _Optional[_Iterable[str]] = ...) -> None: ...

class Capability(_message.Message):
    __slots__ = ("name", "state", "live", "verified_dcs", "live_checks", "since_api_rev", "note", "signature", "superseded_by")
    NAME_FIELD_NUMBER: _ClassVar[int]
    STATE_FIELD_NUMBER: _ClassVar[int]
    LIVE_FIELD_NUMBER: _ClassVar[int]
    VERIFIED_DCS_FIELD_NUMBER: _ClassVar[int]
    LIVE_CHECKS_FIELD_NUMBER: _ClassVar[int]
    SINCE_API_REV_FIELD_NUMBER: _ClassVar[int]
    NOTE_FIELD_NUMBER: _ClassVar[int]
    SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    SUPERSEDED_BY_FIELD_NUMBER: _ClassVar[int]
    name: str
    state: CapabilityState
    live: LiveStatus
    verified_dcs: str
    live_checks: _containers.RepeatedScalarFieldContainer[str]
    since_api_rev: int
    note: str
    signature: CallSignature
    superseded_by: str
    def __init__(self, name: _Optional[str] = ..., state: _Optional[_Union[CapabilityState, str]] = ..., live: _Optional[_Union[LiveStatus, str]] = ..., verified_dcs: _Optional[str] = ..., live_checks: _Optional[_Iterable[str]] = ..., since_api_rev: _Optional[int] = ..., note: _Optional[str] = ..., signature: _Optional[_Union[CallSignature, _Mapping]] = ..., superseded_by: _Optional[str] = ...) -> None: ...
