from sortie_gen._proto.sortie.v1 import common_pb2 as _common_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class Lane(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LANE_UNSPECIFIED: _ClassVar[Lane]
    LANE_CONTROL: _ClassVar[Lane]
    LANE_REALTIME: _ClassVar[Lane]
    LANE_BULK: _ClassVar[Lane]
    LANE_QUERY: _ClassVar[Lane]

class PausePolicy(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    PAUSE_POLICY_UNSPECIFIED: _ClassVar[PausePolicy]
    PAUSE_POLICY_WAIT: _ClassVar[PausePolicy]
    PAUSE_POLICY_FAIL: _ClassVar[PausePolicy]

class OrbitPattern(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    ORBIT_PATTERN_UNSPECIFIED: _ClassVar[OrbitPattern]
    ORBIT_PATTERN_CIRCLE: _ClassVar[OrbitPattern]
    ORBIT_PATTERN_RACE_TRACK: _ClassVar[OrbitPattern]

class MenuScope(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MENU_SCOPE_UNSPECIFIED: _ClassVar[MenuScope]
    MENU_SCOPE_COALITION: _ClassVar[MenuScope]
    MENU_SCOPE_PER_GROUP: _ClassVar[MenuScope]
LANE_UNSPECIFIED: Lane
LANE_CONTROL: Lane
LANE_REALTIME: Lane
LANE_BULK: Lane
LANE_QUERY: Lane
PAUSE_POLICY_UNSPECIFIED: PausePolicy
PAUSE_POLICY_WAIT: PausePolicy
PAUSE_POLICY_FAIL: PausePolicy
ORBIT_PATTERN_UNSPECIFIED: OrbitPattern
ORBIT_PATTERN_CIRCLE: OrbitPattern
ORBIT_PATTERN_RACE_TRACK: OrbitPattern
MENU_SCOPE_UNSPECIFIED: MenuScope
MENU_SCOPE_COALITION: MenuScope
MENU_SCOPE_PER_GROUP: MenuScope

class Origin(_message.Message):
    __slots__ = ("logic", "build_id", "file", "line", "column")
    LOGIC_FIELD_NUMBER: _ClassVar[int]
    BUILD_ID_FIELD_NUMBER: _ClassVar[int]
    FILE_FIELD_NUMBER: _ClassVar[int]
    LINE_FIELD_NUMBER: _ClassVar[int]
    COLUMN_FIELD_NUMBER: _ClassVar[int]
    logic: str
    build_id: str
    file: str
    line: int
    column: int
    def __init__(self, logic: _Optional[str] = ..., build_id: _Optional[str] = ..., file: _Optional[str] = ..., line: _Optional[int] = ..., column: _Optional[int] = ...) -> None: ...

class Command(_message.Message):
    __slots__ = ("id", "key", "lane", "timeout_ms", "on_pause", "after", "origin", "session", "spawn_group", "destroy", "set_task", "set_option", "message", "effect", "flag", "menu", "mark", "end_session", "query", "server", "dev_eval")
    ID_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    LANE_FIELD_NUMBER: _ClassVar[int]
    TIMEOUT_MS_FIELD_NUMBER: _ClassVar[int]
    ON_PAUSE_FIELD_NUMBER: _ClassVar[int]
    AFTER_FIELD_NUMBER: _ClassVar[int]
    ORIGIN_FIELD_NUMBER: _ClassVar[int]
    SESSION_FIELD_NUMBER: _ClassVar[int]
    SPAWN_GROUP_FIELD_NUMBER: _ClassVar[int]
    DESTROY_FIELD_NUMBER: _ClassVar[int]
    SET_TASK_FIELD_NUMBER: _ClassVar[int]
    SET_OPTION_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    EFFECT_FIELD_NUMBER: _ClassVar[int]
    FLAG_FIELD_NUMBER: _ClassVar[int]
    MENU_FIELD_NUMBER: _ClassVar[int]
    MARK_FIELD_NUMBER: _ClassVar[int]
    END_SESSION_FIELD_NUMBER: _ClassVar[int]
    QUERY_FIELD_NUMBER: _ClassVar[int]
    SERVER_FIELD_NUMBER: _ClassVar[int]
    DEV_EVAL_FIELD_NUMBER: _ClassVar[int]
    id: str
    key: str
    lane: Lane
    timeout_ms: int
    on_pause: PausePolicy
    after: str
    origin: Origin
    session: str
    spawn_group: SpawnGroup
    destroy: Destroy
    set_task: SetTask
    set_option: SetOption
    message: Message
    effect: Effect
    flag: Flag
    menu: Menu
    mark: MarkCommand
    end_session: EndSession
    query: Query
    server: ServerControl
    dev_eval: DevEval
    def __init__(self, id: _Optional[str] = ..., key: _Optional[str] = ..., lane: _Optional[_Union[Lane, str]] = ..., timeout_ms: _Optional[int] = ..., on_pause: _Optional[_Union[PausePolicy, str]] = ..., after: _Optional[str] = ..., origin: _Optional[_Union[Origin, _Mapping]] = ..., session: _Optional[str] = ..., spawn_group: _Optional[_Union[SpawnGroup, _Mapping]] = ..., destroy: _Optional[_Union[Destroy, _Mapping]] = ..., set_task: _Optional[_Union[SetTask, _Mapping]] = ..., set_option: _Optional[_Union[SetOption, _Mapping]] = ..., message: _Optional[_Union[Message, _Mapping]] = ..., effect: _Optional[_Union[Effect, _Mapping]] = ..., flag: _Optional[_Union[Flag, _Mapping]] = ..., menu: _Optional[_Union[Menu, _Mapping]] = ..., mark: _Optional[_Union[MarkCommand, _Mapping]] = ..., end_session: _Optional[_Union[EndSession, _Mapping]] = ..., query: _Optional[_Union[Query, _Mapping]] = ..., server: _Optional[_Union[ServerControl, _Mapping]] = ..., dev_eval: _Optional[_Union[DevEval, _Mapping]] = ...) -> None: ...

class CommandResult(_message.Message):
    __slots__ = ("id", "model_time", "cost_us", "ok", "err", "eid")
    ID_FIELD_NUMBER: _ClassVar[int]
    MODEL_TIME_FIELD_NUMBER: _ClassVar[int]
    COST_US_FIELD_NUMBER: _ClassVar[int]
    OK_FIELD_NUMBER: _ClassVar[int]
    ERR_FIELD_NUMBER: _ClassVar[int]
    EID_FIELD_NUMBER: _ClassVar[int]
    id: str
    model_time: float
    cost_us: int
    ok: _common_pb2.LuaValue
    err: _common_pb2.CommandError
    eid: int
    def __init__(self, id: _Optional[str] = ..., model_time: _Optional[float] = ..., cost_us: _Optional[int] = ..., ok: _Optional[_Union[_common_pb2.LuaValue, _Mapping]] = ..., err: _Optional[_Union[_common_pb2.CommandError, _Mapping]] = ..., eid: _Optional[int] = ...) -> None: ...

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

class SpawnGroup(_message.Message):
    __slots__ = ("template", "name", "from_airbase", "at")
    TEMPLATE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    FROM_AIRBASE_FIELD_NUMBER: _ClassVar[int]
    AT_FIELD_NUMBER: _ClassVar[int]
    template: str
    name: str
    from_airbase: FromAirbase
    at: SpawnAt
    def __init__(self, template: _Optional[str] = ..., name: _Optional[str] = ..., from_airbase: _Optional[_Union[FromAirbase, _Mapping]] = ..., at: _Optional[_Union[SpawnAt, _Mapping]] = ...) -> None: ...

class FromAirbase(_message.Message):
    __slots__ = ("airbase", "start")
    AIRBASE_FIELD_NUMBER: _ClassVar[int]
    START_FIELD_NUMBER: _ClassVar[int]
    airbase: str
    start: _common_pb2.StartType
    def __init__(self, airbase: _Optional[str] = ..., start: _Optional[_Union[_common_pb2.StartType, str]] = ...) -> None: ...

class SpawnAt(_message.Message):
    __slots__ = ("position", "heading_rad")
    POSITION_FIELD_NUMBER: _ClassVar[int]
    HEADING_RAD_FIELD_NUMBER: _ClassVar[int]
    position: _common_pb2.Vec3
    heading_rad: float
    def __init__(self, position: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ..., heading_rad: _Optional[float] = ...) -> None: ...

class Destroy(_message.Message):
    __slots__ = ("group", "unit")
    GROUP_FIELD_NUMBER: _ClassVar[int]
    UNIT_FIELD_NUMBER: _ClassVar[int]
    group: GroupSel
    unit: UnitSel
    def __init__(self, group: _Optional[_Union[GroupSel, _Mapping]] = ..., unit: _Optional[_Union[UnitSel, _Mapping]] = ...) -> None: ...

class SetTask(_message.Message):
    __slots__ = ("group", "push", "cap", "orbit", "rtb", "raw")
    GROUP_FIELD_NUMBER: _ClassVar[int]
    PUSH_FIELD_NUMBER: _ClassVar[int]
    CAP_FIELD_NUMBER: _ClassVar[int]
    ORBIT_FIELD_NUMBER: _ClassVar[int]
    RTB_FIELD_NUMBER: _ClassVar[int]
    RAW_FIELD_NUMBER: _ClassVar[int]
    group: GroupSel
    push: bool
    cap: CapTask
    orbit: OrbitTask
    rtb: RtbTask
    raw: _common_pb2.LuaValue
    def __init__(self, group: _Optional[_Union[GroupSel, _Mapping]] = ..., push: _Optional[bool] = ..., cap: _Optional[_Union[CapTask, _Mapping]] = ..., orbit: _Optional[_Union[OrbitTask, _Mapping]] = ..., rtb: _Optional[_Union[RtbTask, _Mapping]] = ..., raw: _Optional[_Union[_common_pb2.LuaValue, _Mapping]] = ...) -> None: ...

class CapTask(_message.Message):
    __slots__ = ("zone", "altitude_m", "speed_mps")
    ZONE_FIELD_NUMBER: _ClassVar[int]
    ALTITUDE_M_FIELD_NUMBER: _ClassVar[int]
    SPEED_MPS_FIELD_NUMBER: _ClassVar[int]
    zone: str
    altitude_m: float
    speed_mps: float
    def __init__(self, zone: _Optional[str] = ..., altitude_m: _Optional[float] = ..., speed_mps: _Optional[float] = ...) -> None: ...

class OrbitTask(_message.Message):
    __slots__ = ("center", "altitude_m", "speed_mps", "pattern", "second")
    CENTER_FIELD_NUMBER: _ClassVar[int]
    ALTITUDE_M_FIELD_NUMBER: _ClassVar[int]
    SPEED_MPS_FIELD_NUMBER: _ClassVar[int]
    PATTERN_FIELD_NUMBER: _ClassVar[int]
    SECOND_FIELD_NUMBER: _ClassVar[int]
    center: _common_pb2.MapPoint
    altitude_m: float
    speed_mps: float
    pattern: OrbitPattern
    second: _common_pb2.MapPoint
    def __init__(self, center: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ..., altitude_m: _Optional[float] = ..., speed_mps: _Optional[float] = ..., pattern: _Optional[_Union[OrbitPattern, str]] = ..., second: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ...) -> None: ...

class RtbTask(_message.Message):
    __slots__ = ("airbase",)
    AIRBASE_FIELD_NUMBER: _ClassVar[int]
    airbase: str
    def __init__(self, airbase: _Optional[str] = ...) -> None: ...

class SetOption(_message.Message):
    __slots__ = ("group", "alarm_state", "roe")
    GROUP_FIELD_NUMBER: _ClassVar[int]
    ALARM_STATE_FIELD_NUMBER: _ClassVar[int]
    ROE_FIELD_NUMBER: _ClassVar[int]
    group: GroupSel
    alarm_state: _common_pb2.AlarmState
    roe: _common_pb2.Roe
    def __init__(self, group: _Optional[_Union[GroupSel, _Mapping]] = ..., alarm_state: _Optional[_Union[_common_pb2.AlarmState, str]] = ..., roe: _Optional[_Union[_common_pb2.Roe, str]] = ...) -> None: ...

class Message(_message.Message):
    __slots__ = ("coalition", "group", "all", "text", "seconds", "clear_view")
    COALITION_FIELD_NUMBER: _ClassVar[int]
    GROUP_FIELD_NUMBER: _ClassVar[int]
    ALL_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    SECONDS_FIELD_NUMBER: _ClassVar[int]
    CLEAR_VIEW_FIELD_NUMBER: _ClassVar[int]
    coalition: _common_pb2.Coalition
    group: GroupSel
    all: bool
    text: str
    seconds: int
    clear_view: bool
    def __init__(self, coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., group: _Optional[_Union[GroupSel, _Mapping]] = ..., all: _Optional[bool] = ..., text: _Optional[str] = ..., seconds: _Optional[int] = ..., clear_view: _Optional[bool] = ...) -> None: ...

class Effect(_message.Message):
    __slots__ = ("smoke",)
    SMOKE_FIELD_NUMBER: _ClassVar[int]
    smoke: Smoke
    def __init__(self, smoke: _Optional[_Union[Smoke, _Mapping]] = ...) -> None: ...

class Smoke(_message.Message):
    __slots__ = ("at", "color", "at_ground_level")
    AT_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    AT_GROUND_LEVEL_FIELD_NUMBER: _ClassVar[int]
    at: _common_pb2.Vec3
    color: _common_pb2.SmokeColor
    at_ground_level: bool
    def __init__(self, at: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ..., color: _Optional[_Union[_common_pb2.SmokeColor, str]] = ..., at_ground_level: _Optional[bool] = ...) -> None: ...

class Flag(_message.Message):
    __slots__ = ("ident", "value")
    IDENT_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    ident: str
    value: int
    def __init__(self, ident: _Optional[str] = ..., value: _Optional[int] = ...) -> None: ...

class Menu(_message.Message):
    __slots__ = ("scope", "coalition", "ident", "label", "path", "remove")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    IDENT_FIELD_NUMBER: _ClassVar[int]
    LABEL_FIELD_NUMBER: _ClassVar[int]
    PATH_FIELD_NUMBER: _ClassVar[int]
    REMOVE_FIELD_NUMBER: _ClassVar[int]
    scope: MenuScope
    coalition: _common_pb2.Coalition
    ident: str
    label: str
    path: _containers.RepeatedScalarFieldContainer[str]
    remove: bool
    def __init__(self, scope: _Optional[_Union[MenuScope, str]] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., ident: _Optional[str] = ..., label: _Optional[str] = ..., path: _Optional[_Iterable[str]] = ..., remove: _Optional[bool] = ...) -> None: ...

class MarkCommand(_message.Message):
    __slots__ = ("name", "text", "at", "coalition", "group", "all", "read_only", "remove")
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    AT_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    GROUP_FIELD_NUMBER: _ClassVar[int]
    ALL_FIELD_NUMBER: _ClassVar[int]
    READ_ONLY_FIELD_NUMBER: _ClassVar[int]
    REMOVE_FIELD_NUMBER: _ClassVar[int]
    name: str
    text: str
    at: _common_pb2.Vec3
    coalition: _common_pb2.Coalition
    group: GroupSel
    all: bool
    read_only: bool
    remove: bool
    def __init__(self, name: _Optional[str] = ..., text: _Optional[str] = ..., at: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., group: _Optional[_Union[GroupSel, _Mapping]] = ..., all: _Optional[bool] = ..., read_only: _Optional[bool] = ..., remove: _Optional[bool] = ...) -> None: ...

class EndSession(_message.Message):
    __slots__ = ("outcome", "text")
    OUTCOME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    outcome: _common_pb2.Outcome
    text: str
    def __init__(self, outcome: _Optional[_Union[_common_pb2.Outcome, str]] = ..., text: _Optional[str] = ...) -> None: ...

class Query(_message.Message):
    __slots__ = ("desc_by_type", "land_height", "flag_value")
    DESC_BY_TYPE_FIELD_NUMBER: _ClassVar[int]
    LAND_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    FLAG_VALUE_FIELD_NUMBER: _ClassVar[int]
    desc_by_type: DescByType
    land_height: LandHeight
    flag_value: FlagValue
    def __init__(self, desc_by_type: _Optional[_Union[DescByType, _Mapping]] = ..., land_height: _Optional[_Union[LandHeight, _Mapping]] = ..., flag_value: _Optional[_Union[FlagValue, _Mapping]] = ...) -> None: ...

class DescByType(_message.Message):
    __slots__ = ("type_name",)
    TYPE_NAME_FIELD_NUMBER: _ClassVar[int]
    type_name: str
    def __init__(self, type_name: _Optional[str] = ...) -> None: ...

class LandHeight(_message.Message):
    __slots__ = ("at",)
    AT_FIELD_NUMBER: _ClassVar[int]
    at: _common_pb2.MapPoint
    def __init__(self, at: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ...) -> None: ...

class FlagValue(_message.Message):
    __slots__ = ("ident",)
    IDENT_FIELD_NUMBER: _ClassVar[int]
    ident: str
    def __init__(self, ident: _Optional[str] = ...) -> None: ...

class ServerControl(_message.Message):
    __slots__ = ("set_pause", "load_mission", "send_chat")
    SET_PAUSE_FIELD_NUMBER: _ClassVar[int]
    LOAD_MISSION_FIELD_NUMBER: _ClassVar[int]
    SEND_CHAT_FIELD_NUMBER: _ClassVar[int]
    set_pause: SetPause
    load_mission: LoadMission
    send_chat: SendChat
    def __init__(self, set_pause: _Optional[_Union[SetPause, _Mapping]] = ..., load_mission: _Optional[_Union[LoadMission, _Mapping]] = ..., send_chat: _Optional[_Union[SendChat, _Mapping]] = ...) -> None: ...

class SetPause(_message.Message):
    __slots__ = ("paused",)
    PAUSED_FIELD_NUMBER: _ClassVar[int]
    paused: bool
    def __init__(self, paused: _Optional[bool] = ...) -> None: ...

class LoadMission(_message.Message):
    __slots__ = ("path",)
    PATH_FIELD_NUMBER: _ClassVar[int]
    path: str
    def __init__(self, path: _Optional[str] = ...) -> None: ...

class SendChat(_message.Message):
    __slots__ = ("text",)
    TEXT_FIELD_NUMBER: _ClassVar[int]
    text: str
    def __init__(self, text: _Optional[str] = ...) -> None: ...

class DevEval(_message.Message):
    __slots__ = ("code", "env")
    CODE_FIELD_NUMBER: _ClassVar[int]
    ENV_FIELD_NUMBER: _ClassVar[int]
    code: str
    env: _common_pb2.Env
    def __init__(self, code: _Optional[str] = ..., env: _Optional[_Union[_common_pb2.Env, str]] = ...) -> None: ...
