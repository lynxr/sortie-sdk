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
    __slots__ = ("code", "message", "lua_traceback")
    CODE_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    LUA_TRACEBACK_FIELD_NUMBER: _ClassVar[int]
    code: ErrorCode
    message: str
    lua_traceback: str
    def __init__(self, code: _Optional[_Union[ErrorCode, str]] = ..., message: _Optional[str] = ..., lua_traceback: _Optional[str] = ...) -> None: ...

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
