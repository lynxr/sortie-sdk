from sortie_gen._proto.sortie.v1 import common_pb2 as _common_pb2
from sortie_gen._proto.sortie.v1 import resources_pb2 as _resources_pb2
from sortie_gen._proto.sortie.v1 import world_pb2 as _world_pb2
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

class SpawnHeadingMode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    SPAWN_HEADING_MODE_UNSPECIFIED: _ClassVar[SpawnHeadingMode]
    SPAWN_HEADING_MODE_FORMATION: _ClassVar[SpawnHeadingMode]

class GroundFormation(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    GROUND_FORMATION_UNSPECIFIED: _ClassVar[GroundFormation]
    GROUND_FORMATION_OFF_ROAD: _ClassVar[GroundFormation]
    GROUND_FORMATION_ON_ROAD: _ClassVar[GroundFormation]
    GROUND_FORMATION_ON_RAILROADS: _ClassVar[GroundFormation]
    GROUND_FORMATION_RANK: _ClassVar[GroundFormation]
    GROUND_FORMATION_CONE: _ClassVar[GroundFormation]
    GROUND_FORMATION_VEE: _ClassVar[GroundFormation]
    GROUND_FORMATION_DIAMOND: _ClassVar[GroundFormation]
    GROUND_FORMATION_ECHELON_LEFT: _ClassVar[GroundFormation]
    GROUND_FORMATION_ECHELON_RIGHT: _ClassVar[GroundFormation]

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

class ObjectCategory(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OBJECT_CATEGORY_UNSPECIFIED: _ClassVar[ObjectCategory]
    OBJECT_CATEGORY_UNIT: _ClassVar[ObjectCategory]
    OBJECT_CATEGORY_WEAPON: _ClassVar[ObjectCategory]
    OBJECT_CATEGORY_STATIC: _ClassVar[ObjectCategory]
    OBJECT_CATEGORY_BASE: _ClassVar[ObjectCategory]
    OBJECT_CATEGORY_SCENERY: _ClassVar[ObjectCategory]
    OBJECT_CATEGORY_CARGO: _ClassVar[ObjectCategory]

class TerminalType(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    TERMINAL_TYPE_UNSPECIFIED: _ClassVar[TerminalType]
    TERMINAL_TYPE_RUNWAY: _ClassVar[TerminalType]
    TERMINAL_TYPE_HELICOPTER_ONLY: _ClassVar[TerminalType]
    TERMINAL_TYPE_SHELTER: _ClassVar[TerminalType]
    TERMINAL_TYPE_OPEN_OR_SHELTER: _ClassVar[TerminalType]
    TERMINAL_TYPE_SMALL_SHELTER: _ClassVar[TerminalType]
    TERMINAL_TYPE_OPEN_AIR: _ClassVar[TerminalType]
    TERMINAL_TYPE_UNKNOWN: _ClassVar[TerminalType]

class StockOutcome(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STOCK_OUTCOME_UNSPECIFIED: _ClassVar[StockOutcome]
    STOCK_OUTCOME_DONE: _ClassVar[StockOutcome]
    STOCK_OUTCOME_NOT_APPLIED: _ClassVar[StockOutcome]
    STOCK_OUTCOME_UNDONE: _ClassVar[StockOutcome]
    STOCK_OUTCOME_UNDO_FAILED: _ClassVar[StockOutcome]
    STOCK_OUTCOME_UNLIMITED: _ClassVar[StockOutcome]
    STOCK_OUTCOME_NOT_ENOUGH: _ClassVar[StockOutcome]
    STOCK_OUTCOME_GONE: _ClassVar[StockOutcome]
    STOCK_OUTCOME_DCS_ERROR: _ClassVar[StockOutcome]
    STOCK_OUTCOME_UNCHANGED: _ClassVar[StockOutcome]
    STOCK_OUTCOME_ALIASED: _ClassVar[StockOutcome]

class ObjectClass(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    OBJECT_CLASS_UNSPECIFIED: _ClassVar[ObjectClass]
    OBJECT_CLASS_UNIT: _ClassVar[ObjectClass]
    OBJECT_CLASS_STATIC: _ClassVar[ObjectClass]
    OBJECT_CLASS_WEAPON: _ClassVar[ObjectClass]
    OBJECT_CLASS_SCENERY: _ClassVar[ObjectClass]
    OBJECT_CLASS_CARGO: _ClassVar[ObjectClass]
    OBJECT_CLASS_AIRBASE: _ClassVar[ObjectClass]
    OBJECT_CLASS_GROUP: _ClassVar[ObjectClass]
    OBJECT_CLASS_CONTROLLER: _ClassVar[ObjectClass]
    OBJECT_CLASS_WAREHOUSE: _ClassVar[ObjectClass]
    OBJECT_CLASS_SPOT: _ClassVar[ObjectClass]
    OBJECT_CLASS_OTHER: _ClassVar[ObjectClass]
LANE_UNSPECIFIED: Lane
LANE_CONTROL: Lane
LANE_REALTIME: Lane
LANE_BULK: Lane
LANE_QUERY: Lane
PAUSE_POLICY_UNSPECIFIED: PausePolicy
PAUSE_POLICY_WAIT: PausePolicy
PAUSE_POLICY_FAIL: PausePolicy
SPAWN_HEADING_MODE_UNSPECIFIED: SpawnHeadingMode
SPAWN_HEADING_MODE_FORMATION: SpawnHeadingMode
GROUND_FORMATION_UNSPECIFIED: GroundFormation
GROUND_FORMATION_OFF_ROAD: GroundFormation
GROUND_FORMATION_ON_ROAD: GroundFormation
GROUND_FORMATION_ON_RAILROADS: GroundFormation
GROUND_FORMATION_RANK: GroundFormation
GROUND_FORMATION_CONE: GroundFormation
GROUND_FORMATION_VEE: GroundFormation
GROUND_FORMATION_DIAMOND: GroundFormation
GROUND_FORMATION_ECHELON_LEFT: GroundFormation
GROUND_FORMATION_ECHELON_RIGHT: GroundFormation
ORBIT_PATTERN_UNSPECIFIED: OrbitPattern
ORBIT_PATTERN_CIRCLE: OrbitPattern
ORBIT_PATTERN_RACE_TRACK: OrbitPattern
MENU_SCOPE_UNSPECIFIED: MenuScope
MENU_SCOPE_COALITION: MenuScope
MENU_SCOPE_PER_GROUP: MenuScope
OBJECT_CATEGORY_UNSPECIFIED: ObjectCategory
OBJECT_CATEGORY_UNIT: ObjectCategory
OBJECT_CATEGORY_WEAPON: ObjectCategory
OBJECT_CATEGORY_STATIC: ObjectCategory
OBJECT_CATEGORY_BASE: ObjectCategory
OBJECT_CATEGORY_SCENERY: ObjectCategory
OBJECT_CATEGORY_CARGO: ObjectCategory
TERMINAL_TYPE_UNSPECIFIED: TerminalType
TERMINAL_TYPE_RUNWAY: TerminalType
TERMINAL_TYPE_HELICOPTER_ONLY: TerminalType
TERMINAL_TYPE_SHELTER: TerminalType
TERMINAL_TYPE_OPEN_OR_SHELTER: TerminalType
TERMINAL_TYPE_SMALL_SHELTER: TerminalType
TERMINAL_TYPE_OPEN_AIR: TerminalType
TERMINAL_TYPE_UNKNOWN: TerminalType
STOCK_OUTCOME_UNSPECIFIED: StockOutcome
STOCK_OUTCOME_DONE: StockOutcome
STOCK_OUTCOME_NOT_APPLIED: StockOutcome
STOCK_OUTCOME_UNDONE: StockOutcome
STOCK_OUTCOME_UNDO_FAILED: StockOutcome
STOCK_OUTCOME_UNLIMITED: StockOutcome
STOCK_OUTCOME_NOT_ENOUGH: StockOutcome
STOCK_OUTCOME_GONE: StockOutcome
STOCK_OUTCOME_DCS_ERROR: StockOutcome
STOCK_OUTCOME_UNCHANGED: StockOutcome
STOCK_OUTCOME_ALIASED: StockOutcome
OBJECT_CLASS_UNSPECIFIED: ObjectClass
OBJECT_CLASS_UNIT: ObjectClass
OBJECT_CLASS_STATIC: ObjectClass
OBJECT_CLASS_WEAPON: ObjectClass
OBJECT_CLASS_SCENERY: ObjectClass
OBJECT_CLASS_CARGO: ObjectClass
OBJECT_CLASS_AIRBASE: ObjectClass
OBJECT_CLASS_GROUP: ObjectClass
OBJECT_CLASS_CONTROLLER: ObjectClass
OBJECT_CLASS_WAREHOUSE: ObjectClass
OBJECT_CLASS_SPOT: ObjectClass
OBJECT_CLASS_OTHER: ObjectClass

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
    __slots__ = ("id", "key", "lane", "timeout_ms", "on_pause", "after", "origin", "session", "min_api_rev", "spawn_group", "spawn_static", "destroy", "dcs_call", "set_task", "dcs_batch", "set_option", "message", "effect", "flag", "menu", "mark", "end_session", "put", "remove", "remove_junk", "query", "server", "spawn_batch", "airbase_control", "warehouse_op", "dev_eval", "api_rev")
    ID_FIELD_NUMBER: _ClassVar[int]
    KEY_FIELD_NUMBER: _ClassVar[int]
    LANE_FIELD_NUMBER: _ClassVar[int]
    TIMEOUT_MS_FIELD_NUMBER: _ClassVar[int]
    ON_PAUSE_FIELD_NUMBER: _ClassVar[int]
    AFTER_FIELD_NUMBER: _ClassVar[int]
    ORIGIN_FIELD_NUMBER: _ClassVar[int]
    SESSION_FIELD_NUMBER: _ClassVar[int]
    MIN_API_REV_FIELD_NUMBER: _ClassVar[int]
    SPAWN_GROUP_FIELD_NUMBER: _ClassVar[int]
    SPAWN_STATIC_FIELD_NUMBER: _ClassVar[int]
    DESTROY_FIELD_NUMBER: _ClassVar[int]
    DCS_CALL_FIELD_NUMBER: _ClassVar[int]
    SET_TASK_FIELD_NUMBER: _ClassVar[int]
    DCS_BATCH_FIELD_NUMBER: _ClassVar[int]
    SET_OPTION_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    EFFECT_FIELD_NUMBER: _ClassVar[int]
    FLAG_FIELD_NUMBER: _ClassVar[int]
    MENU_FIELD_NUMBER: _ClassVar[int]
    MARK_FIELD_NUMBER: _ClassVar[int]
    END_SESSION_FIELD_NUMBER: _ClassVar[int]
    PUT_FIELD_NUMBER: _ClassVar[int]
    REMOVE_FIELD_NUMBER: _ClassVar[int]
    REMOVE_JUNK_FIELD_NUMBER: _ClassVar[int]
    QUERY_FIELD_NUMBER: _ClassVar[int]
    SERVER_FIELD_NUMBER: _ClassVar[int]
    SPAWN_BATCH_FIELD_NUMBER: _ClassVar[int]
    AIRBASE_CONTROL_FIELD_NUMBER: _ClassVar[int]
    WAREHOUSE_OP_FIELD_NUMBER: _ClassVar[int]
    DEV_EVAL_FIELD_NUMBER: _ClassVar[int]
    API_REV_FIELD_NUMBER: _ClassVar[int]
    id: str
    key: str
    lane: Lane
    timeout_ms: int
    on_pause: PausePolicy
    after: str
    origin: Origin
    session: str
    min_api_rev: int
    spawn_group: SpawnGroup
    spawn_static: SpawnStatic
    destroy: Destroy
    dcs_call: DcsCall
    set_task: SetTask
    dcs_batch: DcsBatch
    set_option: SetOption
    message: Message
    effect: Effect
    flag: Flag
    menu: Menu
    mark: MarkCommand
    end_session: EndSession
    put: _resources_pb2.ResourcePut
    remove: _resources_pb2.ResourceRemove
    remove_junk: RemoveJunk
    query: Query
    server: ServerControl
    spawn_batch: SpawnBatch
    airbase_control: AirbaseControl
    warehouse_op: WarehouseOp
    dev_eval: DevEval
    api_rev: int
    def __init__(self, id: _Optional[str] = ..., key: _Optional[str] = ..., lane: _Optional[_Union[Lane, str]] = ..., timeout_ms: _Optional[int] = ..., on_pause: _Optional[_Union[PausePolicy, str]] = ..., after: _Optional[str] = ..., origin: _Optional[_Union[Origin, _Mapping]] = ..., session: _Optional[str] = ..., min_api_rev: _Optional[int] = ..., spawn_group: _Optional[_Union[SpawnGroup, _Mapping]] = ..., spawn_static: _Optional[_Union[SpawnStatic, _Mapping]] = ..., destroy: _Optional[_Union[Destroy, _Mapping]] = ..., dcs_call: _Optional[_Union[DcsCall, _Mapping]] = ..., set_task: _Optional[_Union[SetTask, _Mapping]] = ..., dcs_batch: _Optional[_Union[DcsBatch, _Mapping]] = ..., set_option: _Optional[_Union[SetOption, _Mapping]] = ..., message: _Optional[_Union[Message, _Mapping]] = ..., effect: _Optional[_Union[Effect, _Mapping]] = ..., flag: _Optional[_Union[Flag, _Mapping]] = ..., menu: _Optional[_Union[Menu, _Mapping]] = ..., mark: _Optional[_Union[MarkCommand, _Mapping]] = ..., end_session: _Optional[_Union[EndSession, _Mapping]] = ..., put: _Optional[_Union[_resources_pb2.ResourcePut, _Mapping]] = ..., remove: _Optional[_Union[_resources_pb2.ResourceRemove, _Mapping]] = ..., remove_junk: _Optional[_Union[RemoveJunk, _Mapping]] = ..., query: _Optional[_Union[Query, _Mapping]] = ..., server: _Optional[_Union[ServerControl, _Mapping]] = ..., spawn_batch: _Optional[_Union[SpawnBatch, _Mapping]] = ..., airbase_control: _Optional[_Union[AirbaseControl, _Mapping]] = ..., warehouse_op: _Optional[_Union[WarehouseOp, _Mapping]] = ..., dev_eval: _Optional[_Union[DevEval, _Mapping]] = ..., api_rev: _Optional[int] = ...) -> None: ...

class CommandResult(_message.Message):
    __slots__ = ("id", "model_time", "cost_us", "ok", "err", "eid", "reply", "replayed", "world_rev")
    ID_FIELD_NUMBER: _ClassVar[int]
    MODEL_TIME_FIELD_NUMBER: _ClassVar[int]
    COST_US_FIELD_NUMBER: _ClassVar[int]
    OK_FIELD_NUMBER: _ClassVar[int]
    ERR_FIELD_NUMBER: _ClassVar[int]
    EID_FIELD_NUMBER: _ClassVar[int]
    REPLY_FIELD_NUMBER: _ClassVar[int]
    REPLAYED_FIELD_NUMBER: _ClassVar[int]
    WORLD_REV_FIELD_NUMBER: _ClassVar[int]
    id: str
    model_time: float
    cost_us: int
    ok: _common_pb2.LuaValue
    err: _common_pb2.CommandError
    eid: int
    reply: Reply
    replayed: bool
    world_rev: int
    def __init__(self, id: _Optional[str] = ..., model_time: _Optional[float] = ..., cost_us: _Optional[int] = ..., ok: _Optional[_Union[_common_pb2.LuaValue, _Mapping]] = ..., err: _Optional[_Union[_common_pb2.CommandError, _Mapping]] = ..., eid: _Optional[int] = ..., reply: _Optional[_Union[Reply, _Mapping]] = ..., replayed: _Optional[bool] = ..., world_rev: _Optional[int] = ...) -> None: ...

class Reply(_message.Message):
    __slots__ = ("entities", "groups", "airbases", "resource", "call", "batch", "junk", "spawns", "found", "airbase_control", "airbase_read", "warehouse_stock", "warehouse_op", "item_names")
    ENTITIES_FIELD_NUMBER: _ClassVar[int]
    GROUPS_FIELD_NUMBER: _ClassVar[int]
    AIRBASES_FIELD_NUMBER: _ClassVar[int]
    RESOURCE_FIELD_NUMBER: _ClassVar[int]
    CALL_FIELD_NUMBER: _ClassVar[int]
    BATCH_FIELD_NUMBER: _ClassVar[int]
    JUNK_FIELD_NUMBER: _ClassVar[int]
    SPAWNS_FIELD_NUMBER: _ClassVar[int]
    FOUND_FIELD_NUMBER: _ClassVar[int]
    AIRBASE_CONTROL_FIELD_NUMBER: _ClassVar[int]
    AIRBASE_READ_FIELD_NUMBER: _ClassVar[int]
    WAREHOUSE_STOCK_FIELD_NUMBER: _ClassVar[int]
    WAREHOUSE_OP_FIELD_NUMBER: _ClassVar[int]
    ITEM_NAMES_FIELD_NUMBER: _ClassVar[int]
    entities: _containers.RepeatedCompositeFieldContainer[_world_pb2.EntitySnap]
    groups: _containers.RepeatedCompositeFieldContainer[_world_pb2.GroupSnap]
    airbases: _containers.RepeatedCompositeFieldContainer[_world_pb2.AirbaseSnap]
    resource: _resources_pb2.ResourceAck
    call: DcsReturn
    batch: DcsBatchReturn
    junk: JunkRemoved
    spawns: _containers.RepeatedCompositeFieldContainer[SpawnResult]
    found: FoundObjects
    airbase_control: AirbasesControlled
    airbase_read: AirbasesRead
    warehouse_stock: WarehousesStock
    warehouse_op: StockChanges
    item_names: ItemNameList
    def __init__(self, entities: _Optional[_Iterable[_Union[_world_pb2.EntitySnap, _Mapping]]] = ..., groups: _Optional[_Iterable[_Union[_world_pb2.GroupSnap, _Mapping]]] = ..., airbases: _Optional[_Iterable[_Union[_world_pb2.AirbaseSnap, _Mapping]]] = ..., resource: _Optional[_Union[_resources_pb2.ResourceAck, _Mapping]] = ..., call: _Optional[_Union[DcsReturn, _Mapping]] = ..., batch: _Optional[_Union[DcsBatchReturn, _Mapping]] = ..., junk: _Optional[_Union[JunkRemoved, _Mapping]] = ..., spawns: _Optional[_Iterable[_Union[SpawnResult, _Mapping]]] = ..., found: _Optional[_Union[FoundObjects, _Mapping]] = ..., airbase_control: _Optional[_Union[AirbasesControlled, _Mapping]] = ..., airbase_read: _Optional[_Union[AirbasesRead, _Mapping]] = ..., warehouse_stock: _Optional[_Union[WarehousesStock, _Mapping]] = ..., warehouse_op: _Optional[_Union[StockChanges, _Mapping]] = ..., item_names: _Optional[_Union[ItemNameList, _Mapping]] = ...) -> None: ...

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
    __slots__ = ("position", "heading_rad", "heading_mode")
    POSITION_FIELD_NUMBER: _ClassVar[int]
    HEADING_RAD_FIELD_NUMBER: _ClassVar[int]
    HEADING_MODE_FIELD_NUMBER: _ClassVar[int]
    position: _common_pb2.Vec3
    heading_rad: float
    heading_mode: SpawnHeadingMode
    def __init__(self, position: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ..., heading_rad: _Optional[float] = ..., heading_mode: _Optional[_Union[SpawnHeadingMode, str]] = ...) -> None: ...

class SpawnStatic(_message.Message):
    __slots__ = ("name", "type_name", "country", "at", "on_unit", "dcs_category", "shape_name", "dead", "cargo", "heliport", "livery", "score", "hidden", "on_conflict", "raw")
    NAME_FIELD_NUMBER: _ClassVar[int]
    TYPE_NAME_FIELD_NUMBER: _ClassVar[int]
    COUNTRY_FIELD_NUMBER: _ClassVar[int]
    AT_FIELD_NUMBER: _ClassVar[int]
    ON_UNIT_FIELD_NUMBER: _ClassVar[int]
    DCS_CATEGORY_FIELD_NUMBER: _ClassVar[int]
    SHAPE_NAME_FIELD_NUMBER: _ClassVar[int]
    DEAD_FIELD_NUMBER: _ClassVar[int]
    CARGO_FIELD_NUMBER: _ClassVar[int]
    HELIPORT_FIELD_NUMBER: _ClassVar[int]
    LIVERY_FIELD_NUMBER: _ClassVar[int]
    SCORE_FIELD_NUMBER: _ClassVar[int]
    HIDDEN_FIELD_NUMBER: _ClassVar[int]
    ON_CONFLICT_FIELD_NUMBER: _ClassVar[int]
    RAW_FIELD_NUMBER: _ClassVar[int]
    name: str
    type_name: str
    country: str
    at: StaticAt
    on_unit: OnUnit
    dcs_category: str
    shape_name: str
    dead: bool
    cargo: CargoSpec
    heliport: HeliportSpec
    livery: str
    score: int
    hidden: bool
    on_conflict: _common_pb2.NameConflict
    raw: _common_pb2.LuaValue
    def __init__(self, name: _Optional[str] = ..., type_name: _Optional[str] = ..., country: _Optional[str] = ..., at: _Optional[_Union[StaticAt, _Mapping]] = ..., on_unit: _Optional[_Union[OnUnit, _Mapping]] = ..., dcs_category: _Optional[str] = ..., shape_name: _Optional[str] = ..., dead: _Optional[bool] = ..., cargo: _Optional[_Union[CargoSpec, _Mapping]] = ..., heliport: _Optional[_Union[HeliportSpec, _Mapping]] = ..., livery: _Optional[str] = ..., score: _Optional[int] = ..., hidden: _Optional[bool] = ..., on_conflict: _Optional[_Union[_common_pb2.NameConflict, str]] = ..., raw: _Optional[_Union[_common_pb2.LuaValue, _Mapping]] = ...) -> None: ...

class StaticAt(_message.Message):
    __slots__ = ("position", "heading_rad")
    POSITION_FIELD_NUMBER: _ClassVar[int]
    HEADING_RAD_FIELD_NUMBER: _ClassVar[int]
    position: _common_pb2.MapPoint
    heading_rad: float
    def __init__(self, position: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ..., heading_rad: _Optional[float] = ...) -> None: ...

class OnUnit(_message.Message):
    __slots__ = ("unit", "forward_m", "right_m", "angle_rad")
    UNIT_FIELD_NUMBER: _ClassVar[int]
    FORWARD_M_FIELD_NUMBER: _ClassVar[int]
    RIGHT_M_FIELD_NUMBER: _ClassVar[int]
    ANGLE_RAD_FIELD_NUMBER: _ClassVar[int]
    unit: _common_pb2.UnitSel
    forward_m: float
    right_m: float
    angle_rad: float
    def __init__(self, unit: _Optional[_Union[_common_pb2.UnitSel, _Mapping]] = ..., forward_m: _Optional[float] = ..., right_m: _Optional[float] = ..., angle_rad: _Optional[float] = ...) -> None: ...

class CargoSpec(_message.Message):
    __slots__ = ("mass_kg", "liftable")
    MASS_KG_FIELD_NUMBER: _ClassVar[int]
    LIFTABLE_FIELD_NUMBER: _ClassVar[int]
    mass_kg: float
    liftable: bool
    def __init__(self, mass_kg: _Optional[float] = ..., liftable: _Optional[bool] = ...) -> None: ...

class HeliportSpec(_message.Message):
    __slots__ = ("callsign", "frequency_hz", "modulation")
    CALLSIGN_FIELD_NUMBER: _ClassVar[int]
    FREQUENCY_HZ_FIELD_NUMBER: _ClassVar[int]
    MODULATION_FIELD_NUMBER: _ClassVar[int]
    callsign: str
    frequency_hz: float
    modulation: _common_pb2.Modulation
    def __init__(self, callsign: _Optional[str] = ..., frequency_hz: _Optional[float] = ..., modulation: _Optional[_Union[_common_pb2.Modulation, str]] = ...) -> None: ...

class Destroy(_message.Message):
    __slots__ = ("group", "unit", "object", "allow_player")
    GROUP_FIELD_NUMBER: _ClassVar[int]
    UNIT_FIELD_NUMBER: _ClassVar[int]
    OBJECT_FIELD_NUMBER: _ClassVar[int]
    ALLOW_PLAYER_FIELD_NUMBER: _ClassVar[int]
    group: _common_pb2.GroupSel
    unit: _common_pb2.UnitSel
    object: _common_pb2.ObjSel
    allow_player: bool
    def __init__(self, group: _Optional[_Union[_common_pb2.GroupSel, _Mapping]] = ..., unit: _Optional[_Union[_common_pb2.UnitSel, _Mapping]] = ..., object: _Optional[_Union[_common_pb2.ObjSel, _Mapping]] = ..., allow_player: _Optional[bool] = ...) -> None: ...

class RemoveJunk(_message.Message):
    __slots__ = ("volumes",)
    VOLUMES_FIELD_NUMBER: _ClassVar[int]
    volumes: _containers.RepeatedCompositeFieldContainer[_common_pb2.Volume]
    def __init__(self, volumes: _Optional[_Iterable[_Union[_common_pb2.Volume, _Mapping]]] = ...) -> None: ...

class JunkRemoved(_message.Message):
    __slots__ = ("total", "per_volume")
    TOTAL_FIELD_NUMBER: _ClassVar[int]
    PER_VOLUME_FIELD_NUMBER: _ClassVar[int]
    total: int
    per_volume: _containers.RepeatedScalarFieldContainer[int]
    def __init__(self, total: _Optional[int] = ..., per_volume: _Optional[_Iterable[int]] = ...) -> None: ...

class SpawnBatch(_message.Message):
    __slots__ = ("items", "stop_on_error")
    ITEMS_FIELD_NUMBER: _ClassVar[int]
    STOP_ON_ERROR_FIELD_NUMBER: _ClassVar[int]
    items: _containers.RepeatedCompositeFieldContainer[SpawnItem]
    stop_on_error: bool
    def __init__(self, items: _Optional[_Iterable[_Union[SpawnItem, _Mapping]]] = ..., stop_on_error: _Optional[bool] = ...) -> None: ...

class SpawnItem(_message.Message):
    __slots__ = ("group", "static_object")
    GROUP_FIELD_NUMBER: _ClassVar[int]
    STATIC_OBJECT_FIELD_NUMBER: _ClassVar[int]
    group: SpawnGroup
    static_object: SpawnStatic
    def __init__(self, group: _Optional[_Union[SpawnGroup, _Mapping]] = ..., static_object: _Optional[_Union[SpawnStatic, _Mapping]] = ...) -> None: ...

class SpawnResult(_message.Message):
    __slots__ = ("eid", "name", "err", "skipped", "settled")
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ERR_FIELD_NUMBER: _ClassVar[int]
    SKIPPED_FIELD_NUMBER: _ClassVar[int]
    SETTLED_FIELD_NUMBER: _ClassVar[int]
    eid: int
    name: str
    err: _common_pb2.CommandError
    skipped: bool
    settled: bool
    def __init__(self, eid: _Optional[int] = ..., name: _Optional[str] = ..., err: _Optional[_Union[_common_pb2.CommandError, _Mapping]] = ..., skipped: _Optional[bool] = ..., settled: _Optional[bool] = ...) -> None: ...

class SetTask(_message.Message):
    __slots__ = ("group", "push", "cap", "orbit", "rtb", "ground_route", "raw")
    GROUP_FIELD_NUMBER: _ClassVar[int]
    PUSH_FIELD_NUMBER: _ClassVar[int]
    CAP_FIELD_NUMBER: _ClassVar[int]
    ORBIT_FIELD_NUMBER: _ClassVar[int]
    RTB_FIELD_NUMBER: _ClassVar[int]
    GROUND_ROUTE_FIELD_NUMBER: _ClassVar[int]
    RAW_FIELD_NUMBER: _ClassVar[int]
    group: _common_pb2.GroupSel
    push: bool
    cap: CapTask
    orbit: OrbitTask
    rtb: RtbTask
    ground_route: GroundRoute
    raw: _common_pb2.LuaValue
    def __init__(self, group: _Optional[_Union[_common_pb2.GroupSel, _Mapping]] = ..., push: _Optional[bool] = ..., cap: _Optional[_Union[CapTask, _Mapping]] = ..., orbit: _Optional[_Union[OrbitTask, _Mapping]] = ..., rtb: _Optional[_Union[RtbTask, _Mapping]] = ..., ground_route: _Optional[_Union[GroundRoute, _Mapping]] = ..., raw: _Optional[_Union[_common_pb2.LuaValue, _Mapping]] = ...) -> None: ...

class GroundRoute(_message.Message):
    __slots__ = ("points", "speed_mps")
    POINTS_FIELD_NUMBER: _ClassVar[int]
    SPEED_MPS_FIELD_NUMBER: _ClassVar[int]
    points: _containers.RepeatedCompositeFieldContainer[GroundRoutePoint]
    speed_mps: float
    def __init__(self, points: _Optional[_Iterable[_Union[GroundRoutePoint, _Mapping]]] = ..., speed_mps: _Optional[float] = ...) -> None: ...

class GroundRoutePoint(_message.Message):
    __slots__ = ("point", "formation", "speed_mps")
    POINT_FIELD_NUMBER: _ClassVar[int]
    FORMATION_FIELD_NUMBER: _ClassVar[int]
    SPEED_MPS_FIELD_NUMBER: _ClassVar[int]
    point: _common_pb2.MapPoint
    formation: GroundFormation
    speed_mps: float
    def __init__(self, point: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ..., formation: _Optional[_Union[GroundFormation, str]] = ..., speed_mps: _Optional[float] = ...) -> None: ...

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
    group: _common_pb2.GroupSel
    alarm_state: _common_pb2.AlarmState
    roe: _common_pb2.Roe
    def __init__(self, group: _Optional[_Union[_common_pb2.GroupSel, _Mapping]] = ..., alarm_state: _Optional[_Union[_common_pb2.AlarmState, str]] = ..., roe: _Optional[_Union[_common_pb2.Roe, str]] = ...) -> None: ...

class Message(_message.Message):
    __slots__ = ("coalition", "group", "all", "text", "seconds", "clear_view")
    COALITION_FIELD_NUMBER: _ClassVar[int]
    GROUP_FIELD_NUMBER: _ClassVar[int]
    ALL_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    SECONDS_FIELD_NUMBER: _ClassVar[int]
    CLEAR_VIEW_FIELD_NUMBER: _ClassVar[int]
    coalition: _common_pb2.Coalition
    group: _common_pb2.GroupSel
    all: bool
    text: str
    seconds: int
    clear_view: bool
    def __init__(self, coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., group: _Optional[_Union[_common_pb2.GroupSel, _Mapping]] = ..., all: _Optional[bool] = ..., text: _Optional[str] = ..., seconds: _Optional[int] = ..., clear_view: _Optional[bool] = ...) -> None: ...

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
    group: _common_pb2.GroupSel
    all: bool
    read_only: bool
    remove: bool
    def __init__(self, name: _Optional[str] = ..., text: _Optional[str] = ..., at: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., group: _Optional[_Union[_common_pb2.GroupSel, _Mapping]] = ..., all: _Optional[bool] = ..., read_only: _Optional[bool] = ..., remove: _Optional[bool] = ...) -> None: ...

class EndSession(_message.Message):
    __slots__ = ("outcome", "text")
    OUTCOME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    outcome: _common_pb2.Outcome
    text: str
    def __init__(self, outcome: _Optional[_Union[_common_pb2.Outcome, str]] = ..., text: _Optional[str] = ...) -> None: ...

class Query(_message.Message):
    __slots__ = ("desc_by_type", "land_height", "flag_value", "search_objects", "airbase_parking", "airbase_runways", "airbase_state", "warehouse_stock", "item_names")
    DESC_BY_TYPE_FIELD_NUMBER: _ClassVar[int]
    LAND_HEIGHT_FIELD_NUMBER: _ClassVar[int]
    FLAG_VALUE_FIELD_NUMBER: _ClassVar[int]
    SEARCH_OBJECTS_FIELD_NUMBER: _ClassVar[int]
    AIRBASE_PARKING_FIELD_NUMBER: _ClassVar[int]
    AIRBASE_RUNWAYS_FIELD_NUMBER: _ClassVar[int]
    AIRBASE_STATE_FIELD_NUMBER: _ClassVar[int]
    WAREHOUSE_STOCK_FIELD_NUMBER: _ClassVar[int]
    ITEM_NAMES_FIELD_NUMBER: _ClassVar[int]
    desc_by_type: DescByType
    land_height: LandHeight
    flag_value: FlagValue
    search_objects: SearchObjects
    airbase_parking: AirbaseParking
    airbase_runways: AirbaseRunways
    airbase_state: AirbaseStates
    warehouse_stock: WarehouseStocks
    item_names: ItemNames
    def __init__(self, desc_by_type: _Optional[_Union[DescByType, _Mapping]] = ..., land_height: _Optional[_Union[LandHeight, _Mapping]] = ..., flag_value: _Optional[_Union[FlagValue, _Mapping]] = ..., search_objects: _Optional[_Union[SearchObjects, _Mapping]] = ..., airbase_parking: _Optional[_Union[AirbaseParking, _Mapping]] = ..., airbase_runways: _Optional[_Union[AirbaseRunways, _Mapping]] = ..., airbase_state: _Optional[_Union[AirbaseStates, _Mapping]] = ..., warehouse_stock: _Optional[_Union[WarehouseStocks, _Mapping]] = ..., item_names: _Optional[_Union[ItemNames, _Mapping]] = ...) -> None: ...

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

class SearchObjects(_message.Message):
    __slots__ = ("volume", "categories", "limit")
    VOLUME_FIELD_NUMBER: _ClassVar[int]
    CATEGORIES_FIELD_NUMBER: _ClassVar[int]
    LIMIT_FIELD_NUMBER: _ClassVar[int]
    volume: _common_pb2.Volume
    categories: _containers.RepeatedScalarFieldContainer[ObjectCategory]
    limit: int
    def __init__(self, volume: _Optional[_Union[_common_pb2.Volume, _Mapping]] = ..., categories: _Optional[_Iterable[_Union[ObjectCategory, str]]] = ..., limit: _Optional[int] = ...) -> None: ...

class FoundObjects(_message.Message):
    __slots__ = ("objects", "truncated")
    OBJECTS_FIELD_NUMBER: _ClassVar[int]
    TRUNCATED_FIELD_NUMBER: _ClassVar[int]
    objects: _containers.RepeatedCompositeFieldContainer[FoundObject]
    truncated: bool
    def __init__(self, objects: _Optional[_Iterable[_Union[FoundObject, _Mapping]]] = ..., truncated: _Optional[bool] = ...) -> None: ...

class FoundObject(_message.Message):
    __slots__ = ("category", "eid", "name", "type_name", "point", "life", "life_max", "coalition", "dcs_id")
    CATEGORY_FIELD_NUMBER: _ClassVar[int]
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TYPE_NAME_FIELD_NUMBER: _ClassVar[int]
    POINT_FIELD_NUMBER: _ClassVar[int]
    LIFE_FIELD_NUMBER: _ClassVar[int]
    LIFE_MAX_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    DCS_ID_FIELD_NUMBER: _ClassVar[int]
    category: ObjectCategory
    eid: int
    name: str
    type_name: str
    point: _common_pb2.Vec3
    life: float
    life_max: float
    coalition: _common_pb2.Coalition
    dcs_id: int
    def __init__(self, category: _Optional[_Union[ObjectCategory, str]] = ..., eid: _Optional[int] = ..., name: _Optional[str] = ..., type_name: _Optional[str] = ..., point: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ..., life: _Optional[float] = ..., life_max: _Optional[float] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., dcs_id: _Optional[int] = ...) -> None: ...

class AirbaseControl(_message.Message):
    __slots__ = ("airbases", "coalition", "auto_capture", "radio_silent")
    AIRBASES_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    AUTO_CAPTURE_FIELD_NUMBER: _ClassVar[int]
    RADIO_SILENT_FIELD_NUMBER: _ClassVar[int]
    airbases: _containers.RepeatedCompositeFieldContainer[_common_pb2.AirbaseSel]
    coalition: _common_pb2.Coalition
    auto_capture: bool
    radio_silent: bool
    def __init__(self, airbases: _Optional[_Iterable[_Union[_common_pb2.AirbaseSel, _Mapping]]] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., auto_capture: _Optional[bool] = ..., radio_silent: _Optional[bool] = ...) -> None: ...

class AirbasesControlled(_message.Message):
    __slots__ = ("bases",)
    BASES_FIELD_NUMBER: _ClassVar[int]
    bases: _containers.RepeatedCompositeFieldContainer[AirbaseControlResult]
    def __init__(self, bases: _Optional[_Iterable[_Union[AirbaseControlResult, _Mapping]]] = ...) -> None: ...

class AirbaseControlResult(_message.Message):
    __slots__ = ("eid", "name", "before", "after", "err")
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    BEFORE_FIELD_NUMBER: _ClassVar[int]
    AFTER_FIELD_NUMBER: _ClassVar[int]
    ERR_FIELD_NUMBER: _ClassVar[int]
    eid: int
    name: str
    before: AirbaseControlState
    after: AirbaseControlState
    err: _common_pb2.CommandError
    def __init__(self, eid: _Optional[int] = ..., name: _Optional[str] = ..., before: _Optional[_Union[AirbaseControlState, _Mapping]] = ..., after: _Optional[_Union[AirbaseControlState, _Mapping]] = ..., err: _Optional[_Union[_common_pb2.CommandError, _Mapping]] = ...) -> None: ...

class AirbaseControlState(_message.Message):
    __slots__ = ("coalition", "auto_capture", "radio_silent")
    COALITION_FIELD_NUMBER: _ClassVar[int]
    AUTO_CAPTURE_FIELD_NUMBER: _ClassVar[int]
    RADIO_SILENT_FIELD_NUMBER: _ClassVar[int]
    coalition: _common_pb2.Coalition
    auto_capture: bool
    radio_silent: bool
    def __init__(self, coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., auto_capture: _Optional[bool] = ..., radio_silent: _Optional[bool] = ...) -> None: ...

class AirbaseParking(_message.Message):
    __slots__ = ("airbases", "available_only")
    AIRBASES_FIELD_NUMBER: _ClassVar[int]
    AVAILABLE_ONLY_FIELD_NUMBER: _ClassVar[int]
    airbases: _containers.RepeatedCompositeFieldContainer[_common_pb2.AirbaseSel]
    available_only: bool
    def __init__(self, airbases: _Optional[_Iterable[_Union[_common_pb2.AirbaseSel, _Mapping]]] = ..., available_only: _Optional[bool] = ...) -> None: ...

class AirbaseRunways(_message.Message):
    __slots__ = ("airbases",)
    AIRBASES_FIELD_NUMBER: _ClassVar[int]
    airbases: _containers.RepeatedCompositeFieldContainer[_common_pb2.AirbaseSel]
    def __init__(self, airbases: _Optional[_Iterable[_Union[_common_pb2.AirbaseSel, _Mapping]]] = ...) -> None: ...

class AirbaseStates(_message.Message):
    __slots__ = ("airbases",)
    AIRBASES_FIELD_NUMBER: _ClassVar[int]
    airbases: _containers.RepeatedCompositeFieldContainer[_common_pb2.AirbaseSel]
    def __init__(self, airbases: _Optional[_Iterable[_Union[_common_pb2.AirbaseSel, _Mapping]]] = ...) -> None: ...

class AirbasesRead(_message.Message):
    __slots__ = ("bases",)
    BASES_FIELD_NUMBER: _ClassVar[int]
    bases: _containers.RepeatedCompositeFieldContainer[AirbaseRead]
    def __init__(self, bases: _Optional[_Iterable[_Union[AirbaseRead, _Mapping]]] = ...) -> None: ...

class AirbaseRead(_message.Message):
    __slots__ = ("eid", "name", "err", "state", "parking", "runways")
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    ERR_FIELD_NUMBER: _ClassVar[int]
    STATE_FIELD_NUMBER: _ClassVar[int]
    PARKING_FIELD_NUMBER: _ClassVar[int]
    RUNWAYS_FIELD_NUMBER: _ClassVar[int]
    eid: int
    name: str
    err: _common_pb2.CommandError
    state: AirbaseControlState
    parking: _containers.RepeatedCompositeFieldContainer[ParkingSpot]
    runways: _containers.RepeatedCompositeFieldContainer[Runway]
    def __init__(self, eid: _Optional[int] = ..., name: _Optional[str] = ..., err: _Optional[_Union[_common_pb2.CommandError, _Mapping]] = ..., state: _Optional[_Union[AirbaseControlState, _Mapping]] = ..., parking: _Optional[_Iterable[_Union[ParkingSpot, _Mapping]]] = ..., runways: _Optional[_Iterable[_Union[Runway, _Mapping]]] = ...) -> None: ...

class ParkingSpot(_message.Message):
    __slots__ = ("term_index", "terminal", "terminal_unknown", "point", "taken", "taken_by", "taken_by_name", "dist_to_runway", "to_ac", "term_index_0")
    TERM_INDEX_FIELD_NUMBER: _ClassVar[int]
    TERMINAL_FIELD_NUMBER: _ClassVar[int]
    TERMINAL_UNKNOWN_FIELD_NUMBER: _ClassVar[int]
    POINT_FIELD_NUMBER: _ClassVar[int]
    TAKEN_FIELD_NUMBER: _ClassVar[int]
    TAKEN_BY_FIELD_NUMBER: _ClassVar[int]
    TAKEN_BY_NAME_FIELD_NUMBER: _ClassVar[int]
    DIST_TO_RUNWAY_FIELD_NUMBER: _ClassVar[int]
    TO_AC_FIELD_NUMBER: _ClassVar[int]
    TERM_INDEX_0_FIELD_NUMBER: _ClassVar[int]
    term_index: int
    terminal: TerminalType
    terminal_unknown: str
    point: _common_pb2.Vec3
    taken: bool
    taken_by: int
    taken_by_name: str
    dist_to_runway: float
    to_ac: bool
    term_index_0: int
    def __init__(self, term_index: _Optional[int] = ..., terminal: _Optional[_Union[TerminalType, str]] = ..., terminal_unknown: _Optional[str] = ..., point: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ..., taken: _Optional[bool] = ..., taken_by: _Optional[int] = ..., taken_by_name: _Optional[str] = ..., dist_to_runway: _Optional[float] = ..., to_ac: _Optional[bool] = ..., term_index_0: _Optional[int] = ...) -> None: ...

class Runway(_message.Message):
    __slots__ = ("name", "course", "length", "width", "center")
    NAME_FIELD_NUMBER: _ClassVar[int]
    COURSE_FIELD_NUMBER: _ClassVar[int]
    LENGTH_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    CENTER_FIELD_NUMBER: _ClassVar[int]
    name: str
    course: float
    length: float
    width: float
    center: _common_pb2.Vec3
    def __init__(self, name: _Optional[str] = ..., course: _Optional[float] = ..., length: _Optional[float] = ..., width: _Optional[float] = ..., center: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ...) -> None: ...

class WarehouseStocks(_message.Message):
    __slots__ = ("warehouses", "items", "liquids")
    WAREHOUSES_FIELD_NUMBER: _ClassVar[int]
    ITEMS_FIELD_NUMBER: _ClassVar[int]
    LIQUIDS_FIELD_NUMBER: _ClassVar[int]
    warehouses: _containers.RepeatedCompositeFieldContainer[_common_pb2.WarehouseSel]
    items: _containers.RepeatedCompositeFieldContainer[_common_pb2.WarehouseItem]
    liquids: bool
    def __init__(self, warehouses: _Optional[_Iterable[_Union[_common_pb2.WarehouseSel, _Mapping]]] = ..., items: _Optional[_Iterable[_Union[_common_pb2.WarehouseItem, _Mapping]]] = ..., liquids: _Optional[bool] = ...) -> None: ...

class WarehousesStock(_message.Message):
    __slots__ = ("warehouses",)
    WAREHOUSES_FIELD_NUMBER: _ClassVar[int]
    warehouses: _containers.RepeatedCompositeFieldContainer[WarehouseStock]
    def __init__(self, warehouses: _Optional[_Iterable[_Union[WarehouseStock, _Mapping]]] = ...) -> None: ...

class WarehouseStock(_message.Message):
    __slots__ = ("warehouse", "name", "eid", "err", "aircraft", "weapons", "liquids", "items")
    WAREHOUSE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    EID_FIELD_NUMBER: _ClassVar[int]
    ERR_FIELD_NUMBER: _ClassVar[int]
    AIRCRAFT_FIELD_NUMBER: _ClassVar[int]
    WEAPONS_FIELD_NUMBER: _ClassVar[int]
    LIQUIDS_FIELD_NUMBER: _ClassVar[int]
    ITEMS_FIELD_NUMBER: _ClassVar[int]
    warehouse: _common_pb2.WarehouseSel
    name: str
    eid: int
    err: _common_pb2.CommandError
    aircraft: StockClass
    weapons: StockClass
    liquids: LiquidStock
    items: _containers.RepeatedCompositeFieldContainer[ItemCount]
    def __init__(self, warehouse: _Optional[_Union[_common_pb2.WarehouseSel, _Mapping]] = ..., name: _Optional[str] = ..., eid: _Optional[int] = ..., err: _Optional[_Union[_common_pb2.CommandError, _Mapping]] = ..., aircraft: _Optional[_Union[StockClass, _Mapping]] = ..., weapons: _Optional[_Union[StockClass, _Mapping]] = ..., liquids: _Optional[_Union[LiquidStock, _Mapping]] = ..., items: _Optional[_Iterable[_Union[ItemCount, _Mapping]]] = ...) -> None: ...

class StockClass(_message.Message):
    __slots__ = ("unlimited", "entries")
    UNLIMITED_FIELD_NUMBER: _ClassVar[int]
    ENTRIES_FIELD_NUMBER: _ClassVar[int]
    unlimited: bool
    entries: _containers.RepeatedCompositeFieldContainer[StockEntry]
    def __init__(self, unlimited: _Optional[bool] = ..., entries: _Optional[_Iterable[_Union[StockEntry, _Mapping]]] = ...) -> None: ...

class StockEntry(_message.Message):
    __slots__ = ("name", "count")
    NAME_FIELD_NUMBER: _ClassVar[int]
    COUNT_FIELD_NUMBER: _ClassVar[int]
    name: str
    count: float
    def __init__(self, name: _Optional[str] = ..., count: _Optional[float] = ...) -> None: ...

class LiquidStock(_message.Message):
    __slots__ = ("unlimited", "amounts")
    UNLIMITED_FIELD_NUMBER: _ClassVar[int]
    AMOUNTS_FIELD_NUMBER: _ClassVar[int]
    unlimited: bool
    amounts: _containers.RepeatedCompositeFieldContainer[LiquidAmount]
    def __init__(self, unlimited: _Optional[bool] = ..., amounts: _Optional[_Iterable[_Union[LiquidAmount, _Mapping]]] = ...) -> None: ...

class LiquidAmount(_message.Message):
    __slots__ = ("liquid", "unknown", "amount", "unlimited")
    LIQUID_FIELD_NUMBER: _ClassVar[int]
    UNKNOWN_FIELD_NUMBER: _ClassVar[int]
    AMOUNT_FIELD_NUMBER: _ClassVar[int]
    UNLIMITED_FIELD_NUMBER: _ClassVar[int]
    liquid: _common_pb2.Liquid
    unknown: str
    amount: float
    unlimited: bool
    def __init__(self, liquid: _Optional[_Union[_common_pb2.Liquid, str]] = ..., unknown: _Optional[str] = ..., amount: _Optional[float] = ..., unlimited: _Optional[bool] = ...) -> None: ...

class ItemCount(_message.Message):
    __slots__ = ("item", "count", "unlimited")
    ITEM_FIELD_NUMBER: _ClassVar[int]
    COUNT_FIELD_NUMBER: _ClassVar[int]
    UNLIMITED_FIELD_NUMBER: _ClassVar[int]
    item: _common_pb2.WarehouseItem
    count: float
    unlimited: bool
    def __init__(self, item: _Optional[_Union[_common_pb2.WarehouseItem, _Mapping]] = ..., count: _Optional[float] = ..., unlimited: _Optional[bool] = ...) -> None: ...

class WarehouseOp(_message.Message):
    __slots__ = ("changes",)
    CHANGES_FIELD_NUMBER: _ClassVar[int]
    changes: _containers.RepeatedCompositeFieldContainer[StockChange]
    def __init__(self, changes: _Optional[_Iterable[_Union[StockChange, _Mapping]]] = ...) -> None: ...

class StockChange(_message.Message):
    __slots__ = ("warehouse", "item", "liquid", "set", "add", "remove", "transfer")
    WAREHOUSE_FIELD_NUMBER: _ClassVar[int]
    ITEM_FIELD_NUMBER: _ClassVar[int]
    LIQUID_FIELD_NUMBER: _ClassVar[int]
    SET_FIELD_NUMBER: _ClassVar[int]
    ADD_FIELD_NUMBER: _ClassVar[int]
    REMOVE_FIELD_NUMBER: _ClassVar[int]
    TRANSFER_FIELD_NUMBER: _ClassVar[int]
    warehouse: _common_pb2.WarehouseSel
    item: _common_pb2.WarehouseItem
    liquid: _common_pb2.Liquid
    set: float
    add: float
    remove: float
    transfer: StockTransfer
    def __init__(self, warehouse: _Optional[_Union[_common_pb2.WarehouseSel, _Mapping]] = ..., item: _Optional[_Union[_common_pb2.WarehouseItem, _Mapping]] = ..., liquid: _Optional[_Union[_common_pb2.Liquid, str]] = ..., set: _Optional[float] = ..., add: _Optional[float] = ..., remove: _Optional[float] = ..., transfer: _Optional[_Union[StockTransfer, _Mapping]] = ...) -> None: ...

class StockTransfer(_message.Message):
    __slots__ = ("to", "amount")
    TO_FIELD_NUMBER: _ClassVar[int]
    AMOUNT_FIELD_NUMBER: _ClassVar[int]
    to: _common_pb2.WarehouseSel
    amount: float
    def __init__(self, to: _Optional[_Union[_common_pb2.WarehouseSel, _Mapping]] = ..., amount: _Optional[float] = ...) -> None: ...

class StockChanges(_message.Message):
    __slots__ = ("applied", "undo_failed", "changes")
    APPLIED_FIELD_NUMBER: _ClassVar[int]
    UNDO_FAILED_FIELD_NUMBER: _ClassVar[int]
    CHANGES_FIELD_NUMBER: _ClassVar[int]
    applied: bool
    undo_failed: bool
    changes: _containers.RepeatedCompositeFieldContainer[StockChanged]
    def __init__(self, applied: _Optional[bool] = ..., undo_failed: _Optional[bool] = ..., changes: _Optional[_Iterable[_Union[StockChanged, _Mapping]]] = ...) -> None: ...

class StockChanged(_message.Message):
    __slots__ = ("outcome", "message", "warehouse", "to")
    OUTCOME_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    WAREHOUSE_FIELD_NUMBER: _ClassVar[int]
    TO_FIELD_NUMBER: _ClassVar[int]
    outcome: StockOutcome
    message: str
    warehouse: StockSide
    to: StockSide
    def __init__(self, outcome: _Optional[_Union[StockOutcome, str]] = ..., message: _Optional[str] = ..., warehouse: _Optional[_Union[StockSide, _Mapping]] = ..., to: _Optional[_Union[StockSide, _Mapping]] = ...) -> None: ...

class StockSide(_message.Message):
    __slots__ = ("warehouse", "name", "eid", "before", "after", "unlimited")
    WAREHOUSE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    EID_FIELD_NUMBER: _ClassVar[int]
    BEFORE_FIELD_NUMBER: _ClassVar[int]
    AFTER_FIELD_NUMBER: _ClassVar[int]
    UNLIMITED_FIELD_NUMBER: _ClassVar[int]
    warehouse: _common_pb2.WarehouseSel
    name: str
    eid: int
    before: float
    after: float
    unlimited: bool
    def __init__(self, warehouse: _Optional[_Union[_common_pb2.WarehouseSel, _Mapping]] = ..., name: _Optional[str] = ..., eid: _Optional[int] = ..., before: _Optional[float] = ..., after: _Optional[float] = ..., unlimited: _Optional[bool] = ...) -> None: ...

class ItemNames(_message.Message):
    __slots__ = ("items",)
    ITEMS_FIELD_NUMBER: _ClassVar[int]
    items: _containers.RepeatedCompositeFieldContainer[_common_pb2.WarehouseItem]
    def __init__(self, items: _Optional[_Iterable[_Union[_common_pb2.WarehouseItem, _Mapping]]] = ...) -> None: ...

class ItemNameList(_message.Message):
    __slots__ = ("names", "unknown", "total", "skipped", "dcs_version", "cached", "source")
    NAMES_FIELD_NUMBER: _ClassVar[int]
    UNKNOWN_FIELD_NUMBER: _ClassVar[int]
    TOTAL_FIELD_NUMBER: _ClassVar[int]
    SKIPPED_FIELD_NUMBER: _ClassVar[int]
    DCS_VERSION_FIELD_NUMBER: _ClassVar[int]
    CACHED_FIELD_NUMBER: _ClassVar[int]
    SOURCE_FIELD_NUMBER: _ClassVar[int]
    names: _containers.RepeatedCompositeFieldContainer[ItemName]
    unknown: _containers.RepeatedCompositeFieldContainer[_common_pb2.WarehouseItem]
    total: int
    skipped: int
    dcs_version: str
    cached: bool
    source: str
    def __init__(self, names: _Optional[_Iterable[_Union[ItemName, _Mapping]]] = ..., unknown: _Optional[_Iterable[_Union[_common_pb2.WarehouseItem, _Mapping]]] = ..., total: _Optional[int] = ..., skipped: _Optional[int] = ..., dcs_version: _Optional[str] = ..., cached: _Optional[bool] = ..., source: _Optional[str] = ...) -> None: ...

class ItemName(_message.Message):
    __slots__ = ("name", "ws_type")
    NAME_FIELD_NUMBER: _ClassVar[int]
    WS_TYPE_FIELD_NUMBER: _ClassVar[int]
    name: str
    ws_type: _common_pb2.WsType
    def __init__(self, name: _Optional[str] = ..., ws_type: _Optional[_Union[_common_pb2.WsType, _Mapping]] = ...) -> None: ...

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

class DcsRef(_message.Message):
    __slots__ = ("object", "group", "airbase", "controller", "warehouse", "handle", "result_of")
    OBJECT_FIELD_NUMBER: _ClassVar[int]
    GROUP_FIELD_NUMBER: _ClassVar[int]
    AIRBASE_FIELD_NUMBER: _ClassVar[int]
    CONTROLLER_FIELD_NUMBER: _ClassVar[int]
    WAREHOUSE_FIELD_NUMBER: _ClassVar[int]
    HANDLE_FIELD_NUMBER: _ClassVar[int]
    RESULT_OF_FIELD_NUMBER: _ClassVar[int]
    object: _common_pb2.ObjSel
    group: _common_pb2.GroupSel
    airbase: _common_pb2.AirbaseSel
    controller: _common_pb2.ControllerSel
    warehouse: _common_pb2.WarehouseSel
    handle: int
    result_of: int
    def __init__(self, object: _Optional[_Union[_common_pb2.ObjSel, _Mapping]] = ..., group: _Optional[_Union[_common_pb2.GroupSel, _Mapping]] = ..., airbase: _Optional[_Union[_common_pb2.AirbaseSel, _Mapping]] = ..., controller: _Optional[_Union[_common_pb2.ControllerSel, _Mapping]] = ..., warehouse: _Optional[_Union[_common_pb2.WarehouseSel, _Mapping]] = ..., handle: _Optional[int] = ..., result_of: _Optional[int] = ...) -> None: ...

class DcsArg(_message.Message):
    __slots__ = ("value", "object", "id_of", "name_of", "enum_key", "coalition", "vec3", "vec2", "player", "table")
    VALUE_FIELD_NUMBER: _ClassVar[int]
    OBJECT_FIELD_NUMBER: _ClassVar[int]
    ID_OF_FIELD_NUMBER: _ClassVar[int]
    NAME_OF_FIELD_NUMBER: _ClassVar[int]
    ENUM_KEY_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    VEC3_FIELD_NUMBER: _ClassVar[int]
    VEC2_FIELD_NUMBER: _ClassVar[int]
    PLAYER_FIELD_NUMBER: _ClassVar[int]
    TABLE_FIELD_NUMBER: _ClassVar[int]
    value: _common_pb2.LuaValue
    object: DcsRef
    id_of: DcsRef
    name_of: DcsRef
    enum_key: str
    coalition: _common_pb2.Coalition
    vec3: _common_pb2.Vec3
    vec2: _common_pb2.MapPoint
    player: _common_pb2.PlayerSel
    table: DcsTable
    def __init__(self, value: _Optional[_Union[_common_pb2.LuaValue, _Mapping]] = ..., object: _Optional[_Union[DcsRef, _Mapping]] = ..., id_of: _Optional[_Union[DcsRef, _Mapping]] = ..., name_of: _Optional[_Union[DcsRef, _Mapping]] = ..., enum_key: _Optional[str] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., vec3: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ..., vec2: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ..., player: _Optional[_Union[_common_pb2.PlayerSel, _Mapping]] = ..., table: _Optional[_Union[DcsTable, _Mapping]] = ...) -> None: ...

class DcsTable(_message.Message):
    __slots__ = ("fields",)
    FIELDS_FIELD_NUMBER: _ClassVar[int]
    fields: _containers.RepeatedCompositeFieldContainer[DcsField]
    def __init__(self, fields: _Optional[_Iterable[_Union[DcsField, _Mapping]]] = ...) -> None: ...

class DcsField(_message.Message):
    __slots__ = ("key", "index", "value")
    KEY_FIELD_NUMBER: _ClassVar[int]
    INDEX_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    key: str
    index: int
    value: DcsArg
    def __init__(self, key: _Optional[str] = ..., index: _Optional[int] = ..., value: _Optional[_Union[DcsArg, _Mapping]] = ...) -> None: ...

class DcsCall(_message.Message):
    __slots__ = ("path", "env", "receiver", "args", "max_depth")
    PATH_FIELD_NUMBER: _ClassVar[int]
    ENV_FIELD_NUMBER: _ClassVar[int]
    RECEIVER_FIELD_NUMBER: _ClassVar[int]
    ARGS_FIELD_NUMBER: _ClassVar[int]
    MAX_DEPTH_FIELD_NUMBER: _ClassVar[int]
    path: str
    env: _common_pb2.Env
    receiver: DcsRef
    args: _containers.RepeatedCompositeFieldContainer[DcsArg]
    max_depth: int
    def __init__(self, path: _Optional[str] = ..., env: _Optional[_Union[_common_pb2.Env, str]] = ..., receiver: _Optional[_Union[DcsRef, _Mapping]] = ..., args: _Optional[_Iterable[_Union[DcsArg, _Mapping]]] = ..., max_depth: _Optional[int] = ...) -> None: ...

class DcsBatch(_message.Message):
    __slots__ = ("calls", "stop_on_error", "fail_command_on_error")
    CALLS_FIELD_NUMBER: _ClassVar[int]
    STOP_ON_ERROR_FIELD_NUMBER: _ClassVar[int]
    FAIL_COMMAND_ON_ERROR_FIELD_NUMBER: _ClassVar[int]
    calls: _containers.RepeatedCompositeFieldContainer[DcsCall]
    stop_on_error: bool
    fail_command_on_error: bool
    def __init__(self, calls: _Optional[_Iterable[_Union[DcsCall, _Mapping]]] = ..., stop_on_error: _Optional[bool] = ..., fail_command_on_error: _Optional[bool] = ...) -> None: ...

class ObjectRef(_message.Message):
    __slots__ = ("object_class", "eid", "name", "type_name", "handle", "controller", "warehouse", "entity", "group", "airbase", "weapon")
    OBJECT_CLASS_FIELD_NUMBER: _ClassVar[int]
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TYPE_NAME_FIELD_NUMBER: _ClassVar[int]
    HANDLE_FIELD_NUMBER: _ClassVar[int]
    CONTROLLER_FIELD_NUMBER: _ClassVar[int]
    WAREHOUSE_FIELD_NUMBER: _ClassVar[int]
    ENTITY_FIELD_NUMBER: _ClassVar[int]
    GROUP_FIELD_NUMBER: _ClassVar[int]
    AIRBASE_FIELD_NUMBER: _ClassVar[int]
    WEAPON_FIELD_NUMBER: _ClassVar[int]
    object_class: ObjectClass
    eid: int
    name: str
    type_name: str
    handle: int
    controller: _common_pb2.ControllerSel
    warehouse: _common_pb2.WarehouseSel
    entity: _world_pb2.EntitySnap
    group: _world_pb2.GroupSnap
    airbase: _world_pb2.AirbaseSnap
    weapon: _world_pb2.WeaponSnap
    def __init__(self, object_class: _Optional[_Union[ObjectClass, str]] = ..., eid: _Optional[int] = ..., name: _Optional[str] = ..., type_name: _Optional[str] = ..., handle: _Optional[int] = ..., controller: _Optional[_Union[_common_pb2.ControllerSel, _Mapping]] = ..., warehouse: _Optional[_Union[_common_pb2.WarehouseSel, _Mapping]] = ..., entity: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ..., group: _Optional[_Union[_world_pb2.GroupSnap, _Mapping]] = ..., airbase: _Optional[_Union[_world_pb2.AirbaseSnap, _Mapping]] = ..., weapon: _Optional[_Union[_world_pb2.WeaponSnap, _Mapping]] = ...) -> None: ...

class DcsValue(_message.Message):
    __slots__ = ("nil", "boolean", "number", "string", "table", "ref", "enum_key", "cut")
    NIL_FIELD_NUMBER: _ClassVar[int]
    BOOLEAN_FIELD_NUMBER: _ClassVar[int]
    NUMBER_FIELD_NUMBER: _ClassVar[int]
    STRING_FIELD_NUMBER: _ClassVar[int]
    TABLE_FIELD_NUMBER: _ClassVar[int]
    REF_FIELD_NUMBER: _ClassVar[int]
    ENUM_KEY_FIELD_NUMBER: _ClassVar[int]
    CUT_FIELD_NUMBER: _ClassVar[int]
    nil: _common_pb2.LuaNil
    boolean: bool
    number: float
    string: str
    table: DcsValueTable
    ref: int
    enum_key: str
    cut: int
    def __init__(self, nil: _Optional[_Union[_common_pb2.LuaNil, _Mapping]] = ..., boolean: _Optional[bool] = ..., number: _Optional[float] = ..., string: _Optional[str] = ..., table: _Optional[_Union[DcsValueTable, _Mapping]] = ..., ref: _Optional[int] = ..., enum_key: _Optional[str] = ..., cut: _Optional[int] = ...) -> None: ...

class DcsValueTable(_message.Message):
    __slots__ = ("pairs",)
    PAIRS_FIELD_NUMBER: _ClassVar[int]
    pairs: _containers.RepeatedCompositeFieldContainer[DcsValuePair]
    def __init__(self, pairs: _Optional[_Iterable[_Union[DcsValuePair, _Mapping]]] = ...) -> None: ...

class DcsValuePair(_message.Message):
    __slots__ = ("key", "value")
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    key: DcsValue
    value: DcsValue
    def __init__(self, key: _Optional[_Union[DcsValue, _Mapping]] = ..., value: _Optional[_Union[DcsValue, _Mapping]] = ...) -> None: ...

class DcsReturn(_message.Message):
    __slots__ = ("values", "refs", "call_class", "truncated")
    VALUES_FIELD_NUMBER: _ClassVar[int]
    REFS_FIELD_NUMBER: _ClassVar[int]
    CALL_CLASS_FIELD_NUMBER: _ClassVar[int]
    TRUNCATED_FIELD_NUMBER: _ClassVar[int]
    values: _containers.RepeatedCompositeFieldContainer[DcsValue]
    refs: _containers.RepeatedCompositeFieldContainer[ObjectRef]
    call_class: _common_pb2.CallClass
    truncated: bool
    def __init__(self, values: _Optional[_Iterable[_Union[DcsValue, _Mapping]]] = ..., refs: _Optional[_Iterable[_Union[ObjectRef, _Mapping]]] = ..., call_class: _Optional[_Union[_common_pb2.CallClass, str]] = ..., truncated: _Optional[bool] = ...) -> None: ...

class DcsCallResult(_message.Message):
    __slots__ = ("ok", "err", "cost_us", "skipped")
    OK_FIELD_NUMBER: _ClassVar[int]
    ERR_FIELD_NUMBER: _ClassVar[int]
    COST_US_FIELD_NUMBER: _ClassVar[int]
    SKIPPED_FIELD_NUMBER: _ClassVar[int]
    ok: DcsReturn
    err: _common_pb2.CommandError
    cost_us: int
    skipped: bool
    def __init__(self, ok: _Optional[_Union[DcsReturn, _Mapping]] = ..., err: _Optional[_Union[_common_pb2.CommandError, _Mapping]] = ..., cost_us: _Optional[int] = ..., skipped: _Optional[bool] = ...) -> None: ...

class DcsBatchReturn(_message.Message):
    __slots__ = ("results",)
    RESULTS_FIELD_NUMBER: _ClassVar[int]
    results: _containers.RepeatedCompositeFieldContainer[DcsCallResult]
    def __init__(self, results: _Optional[_Iterable[_Union[DcsCallResult, _Mapping]]] = ...) -> None: ...
