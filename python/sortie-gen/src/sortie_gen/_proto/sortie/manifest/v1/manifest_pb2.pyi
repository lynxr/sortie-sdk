from sortie_gen._proto.sortie.v1 import common_pb2 as _common_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class Manifest(_message.Message):
    __slots__ = ("scenario", "schema", "schema_sha256", "build_id", "theatre", "start_time", "time_limit_s", "groups", "zones", "templates", "flags", "airbases", "menus", "player_flights", "max_ids", "tools")
    SCENARIO_FIELD_NUMBER: _ClassVar[int]
    SCHEMA_FIELD_NUMBER: _ClassVar[int]
    SCHEMA_SHA256_FIELD_NUMBER: _ClassVar[int]
    BUILD_ID_FIELD_NUMBER: _ClassVar[int]
    THEATRE_FIELD_NUMBER: _ClassVar[int]
    START_TIME_FIELD_NUMBER: _ClassVar[int]
    TIME_LIMIT_S_FIELD_NUMBER: _ClassVar[int]
    GROUPS_FIELD_NUMBER: _ClassVar[int]
    ZONES_FIELD_NUMBER: _ClassVar[int]
    TEMPLATES_FIELD_NUMBER: _ClassVar[int]
    FLAGS_FIELD_NUMBER: _ClassVar[int]
    AIRBASES_FIELD_NUMBER: _ClassVar[int]
    MENUS_FIELD_NUMBER: _ClassVar[int]
    PLAYER_FLIGHTS_FIELD_NUMBER: _ClassVar[int]
    MAX_IDS_FIELD_NUMBER: _ClassVar[int]
    TOOLS_FIELD_NUMBER: _ClassVar[int]
    scenario: str
    schema: int
    schema_sha256: str
    build_id: str
    theatre: str
    start_time: str
    time_limit_s: int
    groups: _containers.RepeatedCompositeFieldContainer[Group]
    zones: _containers.RepeatedCompositeFieldContainer[Zone]
    templates: _containers.RepeatedCompositeFieldContainer[Template]
    flags: _containers.RepeatedCompositeFieldContainer[Flag]
    airbases: _containers.RepeatedCompositeFieldContainer[Airbase]
    menus: _containers.RepeatedCompositeFieldContainer[Menu]
    player_flights: _containers.RepeatedCompositeFieldContainer[PlayerFlight]
    max_ids: MaxIds
    tools: ToolVersions
    def __init__(self, scenario: _Optional[str] = ..., schema: _Optional[int] = ..., schema_sha256: _Optional[str] = ..., build_id: _Optional[str] = ..., theatre: _Optional[str] = ..., start_time: _Optional[str] = ..., time_limit_s: _Optional[int] = ..., groups: _Optional[_Iterable[_Union[Group, _Mapping]]] = ..., zones: _Optional[_Iterable[_Union[Zone, _Mapping]]] = ..., templates: _Optional[_Iterable[_Union[Template, _Mapping]]] = ..., flags: _Optional[_Iterable[_Union[Flag, _Mapping]]] = ..., airbases: _Optional[_Iterable[_Union[Airbase, _Mapping]]] = ..., menus: _Optional[_Iterable[_Union[Menu, _Mapping]]] = ..., player_flights: _Optional[_Iterable[_Union[PlayerFlight, _Mapping]]] = ..., max_ids: _Optional[_Union[MaxIds, _Mapping]] = ..., tools: _Optional[_Union[ToolVersions, _Mapping]] = ...) -> None: ...

class Group(_message.Message):
    __slots__ = ("ident", "name", "category", "coalition", "country", "country_id", "roles", "units", "group_id", "late_activation")
    IDENT_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    CATEGORY_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    COUNTRY_FIELD_NUMBER: _ClassVar[int]
    COUNTRY_ID_FIELD_NUMBER: _ClassVar[int]
    ROLES_FIELD_NUMBER: _ClassVar[int]
    UNITS_FIELD_NUMBER: _ClassVar[int]
    GROUP_ID_FIELD_NUMBER: _ClassVar[int]
    LATE_ACTIVATION_FIELD_NUMBER: _ClassVar[int]
    ident: str
    name: str
    category: _common_pb2.Category
    coalition: _common_pb2.Coalition
    country: str
    country_id: int
    roles: _containers.RepeatedScalarFieldContainer[str]
    units: _containers.RepeatedCompositeFieldContainer[Unit]
    group_id: int
    late_activation: bool
    def __init__(self, ident: _Optional[str] = ..., name: _Optional[str] = ..., category: _Optional[_Union[_common_pb2.Category, str]] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., country: _Optional[str] = ..., country_id: _Optional[int] = ..., roles: _Optional[_Iterable[str]] = ..., units: _Optional[_Iterable[_Union[Unit, _Mapping]]] = ..., group_id: _Optional[int] = ..., late_activation: _Optional[bool] = ...) -> None: ...

class Unit(_message.Message):
    __slots__ = ("name", "type_name", "position", "alt_m", "heading_rad", "unit_id", "latlon", "skill")
    NAME_FIELD_NUMBER: _ClassVar[int]
    TYPE_NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    ALT_M_FIELD_NUMBER: _ClassVar[int]
    HEADING_RAD_FIELD_NUMBER: _ClassVar[int]
    UNIT_ID_FIELD_NUMBER: _ClassVar[int]
    LATLON_FIELD_NUMBER: _ClassVar[int]
    SKILL_FIELD_NUMBER: _ClassVar[int]
    name: str
    type_name: str
    position: _common_pb2.MapPoint
    alt_m: float
    heading_rad: float
    unit_id: int
    latlon: _common_pb2.LatLon
    skill: str
    def __init__(self, name: _Optional[str] = ..., type_name: _Optional[str] = ..., position: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ..., alt_m: _Optional[float] = ..., heading_rad: _Optional[float] = ..., unit_id: _Optional[int] = ..., latlon: _Optional[_Union[_common_pb2.LatLon, _Mapping]] = ..., skill: _Optional[str] = ...) -> None: ...

class Zone(_message.Message):
    __slots__ = ("ident", "name", "circle", "polygon")
    IDENT_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    CIRCLE_FIELD_NUMBER: _ClassVar[int]
    POLYGON_FIELD_NUMBER: _ClassVar[int]
    ident: str
    name: str
    circle: Circle
    polygon: Polygon
    def __init__(self, ident: _Optional[str] = ..., name: _Optional[str] = ..., circle: _Optional[_Union[Circle, _Mapping]] = ..., polygon: _Optional[_Union[Polygon, _Mapping]] = ...) -> None: ...

class Circle(_message.Message):
    __slots__ = ("center", "radius_m")
    CENTER_FIELD_NUMBER: _ClassVar[int]
    RADIUS_M_FIELD_NUMBER: _ClassVar[int]
    center: _common_pb2.MapPoint
    radius_m: float
    def __init__(self, center: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ..., radius_m: _Optional[float] = ...) -> None: ...

class Polygon(_message.Message):
    __slots__ = ("points",)
    POINTS_FIELD_NUMBER: _ClassVar[int]
    points: _containers.RepeatedCompositeFieldContainer[_common_pb2.MapPoint]
    def __init__(self, points: _Optional[_Iterable[_Union[_common_pb2.MapPoint, _Mapping]]] = ...) -> None: ...

class Template(_message.Message):
    __slots__ = ("ident", "name", "category", "coalition", "country", "country_id", "roles", "units", "task", "dcs_group_json")
    IDENT_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    CATEGORY_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    COUNTRY_FIELD_NUMBER: _ClassVar[int]
    COUNTRY_ID_FIELD_NUMBER: _ClassVar[int]
    ROLES_FIELD_NUMBER: _ClassVar[int]
    UNITS_FIELD_NUMBER: _ClassVar[int]
    TASK_FIELD_NUMBER: _ClassVar[int]
    DCS_GROUP_JSON_FIELD_NUMBER: _ClassVar[int]
    ident: str
    name: str
    category: _common_pb2.Category
    coalition: _common_pb2.Coalition
    country: str
    country_id: int
    roles: _containers.RepeatedScalarFieldContainer[str]
    units: _containers.RepeatedCompositeFieldContainer[Unit]
    task: str
    dcs_group_json: str
    def __init__(self, ident: _Optional[str] = ..., name: _Optional[str] = ..., category: _Optional[_Union[_common_pb2.Category, str]] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., country: _Optional[str] = ..., country_id: _Optional[int] = ..., roles: _Optional[_Iterable[str]] = ..., units: _Optional[_Iterable[_Union[Unit, _Mapping]]] = ..., task: _Optional[str] = ..., dcs_group_json: _Optional[str] = ...) -> None: ...

class Flag(_message.Message):
    __slots__ = ("ident", "name", "initial")
    IDENT_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    INITIAL_FIELD_NUMBER: _ClassVar[int]
    ident: str
    name: str
    initial: int
    def __init__(self, ident: _Optional[str] = ..., name: _Optional[str] = ..., initial: _Optional[int] = ...) -> None: ...

class Airbase(_message.Message):
    __slots__ = ("ident", "name", "position", "airbase_id", "coalition")
    IDENT_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    AIRBASE_ID_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    ident: str
    name: str
    position: _common_pb2.MapPoint
    airbase_id: int
    coalition: _common_pb2.Coalition
    def __init__(self, ident: _Optional[str] = ..., name: _Optional[str] = ..., position: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ..., airbase_id: _Optional[int] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ...) -> None: ...

class Menu(_message.Message):
    __slots__ = ("ident", "label", "path")
    IDENT_FIELD_NUMBER: _ClassVar[int]
    LABEL_FIELD_NUMBER: _ClassVar[int]
    PATH_FIELD_NUMBER: _ClassVar[int]
    ident: str
    label: str
    path: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, ident: _Optional[str] = ..., label: _Optional[str] = ..., path: _Optional[_Iterable[str]] = ...) -> None: ...

class PlayerFlight(_message.Message):
    __slots__ = ("name", "type_name", "size", "coalition", "country", "country_id", "airbase", "start", "units", "group_id")
    NAME_FIELD_NUMBER: _ClassVar[int]
    TYPE_NAME_FIELD_NUMBER: _ClassVar[int]
    SIZE_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    COUNTRY_FIELD_NUMBER: _ClassVar[int]
    COUNTRY_ID_FIELD_NUMBER: _ClassVar[int]
    AIRBASE_FIELD_NUMBER: _ClassVar[int]
    START_FIELD_NUMBER: _ClassVar[int]
    UNITS_FIELD_NUMBER: _ClassVar[int]
    GROUP_ID_FIELD_NUMBER: _ClassVar[int]
    name: str
    type_name: str
    size: int
    coalition: _common_pb2.Coalition
    country: str
    country_id: int
    airbase: str
    start: _common_pb2.StartType
    units: _containers.RepeatedCompositeFieldContainer[Unit]
    group_id: int
    def __init__(self, name: _Optional[str] = ..., type_name: _Optional[str] = ..., size: _Optional[int] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., country: _Optional[str] = ..., country_id: _Optional[int] = ..., airbase: _Optional[str] = ..., start: _Optional[_Union[_common_pb2.StartType, str]] = ..., units: _Optional[_Iterable[_Union[Unit, _Mapping]]] = ..., group_id: _Optional[int] = ...) -> None: ...

class MaxIds(_message.Message):
    __slots__ = ("group_id", "unit_id")
    GROUP_ID_FIELD_NUMBER: _ClassVar[int]
    UNIT_ID_FIELD_NUMBER: _ClassVar[int]
    group_id: int
    unit_id: int
    def __init__(self, group_id: _Optional[int] = ..., unit_id: _Optional[int] = ...) -> None: ...

class ToolVersions(_message.Message):
    __slots__ = ("generator", "pydcs", "python", "extra")
    class ExtraEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    GENERATOR_FIELD_NUMBER: _ClassVar[int]
    PYDCS_FIELD_NUMBER: _ClassVar[int]
    PYTHON_FIELD_NUMBER: _ClassVar[int]
    EXTRA_FIELD_NUMBER: _ClassVar[int]
    generator: str
    pydcs: str
    python: str
    extra: _containers.ScalarMap[str, str]
    def __init__(self, generator: _Optional[str] = ..., pydcs: _Optional[str] = ..., python: _Optional[str] = ..., extra: _Optional[_Mapping[str, str]] = ...) -> None: ...
