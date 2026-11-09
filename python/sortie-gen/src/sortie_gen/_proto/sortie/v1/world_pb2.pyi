from google.protobuf import field_mask_pb2 as _field_mask_pb2
from sortie_gen._proto.sortie.v1 import common_pb2 as _common_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class EntitySnap(_message.Message):
    __slots__ = ("eid", "name", "type_name", "coalition", "category", "pos", "player", "group_eid", "country", "velocity", "heading_rad", "dead", "runtime_id", "unit_id", "group_name", "life")
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TYPE_NAME_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    CATEGORY_FIELD_NUMBER: _ClassVar[int]
    POS_FIELD_NUMBER: _ClassVar[int]
    PLAYER_FIELD_NUMBER: _ClassVar[int]
    GROUP_EID_FIELD_NUMBER: _ClassVar[int]
    COUNTRY_FIELD_NUMBER: _ClassVar[int]
    VELOCITY_FIELD_NUMBER: _ClassVar[int]
    HEADING_RAD_FIELD_NUMBER: _ClassVar[int]
    DEAD_FIELD_NUMBER: _ClassVar[int]
    RUNTIME_ID_FIELD_NUMBER: _ClassVar[int]
    UNIT_ID_FIELD_NUMBER: _ClassVar[int]
    GROUP_NAME_FIELD_NUMBER: _ClassVar[int]
    LIFE_FIELD_NUMBER: _ClassVar[int]
    eid: int
    name: str
    type_name: str
    coalition: _common_pb2.Coalition
    category: _common_pb2.Category
    pos: _common_pb2.Vec3
    player: str
    group_eid: int
    country: str
    velocity: _common_pb2.Vec3
    heading_rad: float
    dead: bool
    runtime_id: int
    unit_id: int
    group_name: str
    life: float
    def __init__(self, eid: _Optional[int] = ..., name: _Optional[str] = ..., type_name: _Optional[str] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., category: _Optional[_Union[_common_pb2.Category, str]] = ..., pos: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ..., player: _Optional[str] = ..., group_eid: _Optional[int] = ..., country: _Optional[str] = ..., velocity: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ..., heading_rad: _Optional[float] = ..., dead: _Optional[bool] = ..., runtime_id: _Optional[int] = ..., unit_id: _Optional[int] = ..., group_name: _Optional[str] = ..., life: _Optional[float] = ...) -> None: ...

class GroupSnap(_message.Message):
    __slots__ = ("eid", "name", "ident", "coalition", "category", "units", "group_id", "country")
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    IDENT_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    CATEGORY_FIELD_NUMBER: _ClassVar[int]
    UNITS_FIELD_NUMBER: _ClassVar[int]
    GROUP_ID_FIELD_NUMBER: _ClassVar[int]
    COUNTRY_FIELD_NUMBER: _ClassVar[int]
    eid: int
    name: str
    ident: str
    coalition: _common_pb2.Coalition
    category: _common_pb2.Category
    units: _containers.RepeatedScalarFieldContainer[int]
    group_id: int
    country: str
    def __init__(self, eid: _Optional[int] = ..., name: _Optional[str] = ..., ident: _Optional[str] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., category: _Optional[_Union[_common_pb2.Category, str]] = ..., units: _Optional[_Iterable[int]] = ..., group_id: _Optional[int] = ..., country: _Optional[str] = ...) -> None: ...

class AirbaseSnap(_message.Message):
    __slots__ = ("eid", "name", "ident", "coalition", "pos", "airbase_id")
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    IDENT_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    POS_FIELD_NUMBER: _ClassVar[int]
    AIRBASE_ID_FIELD_NUMBER: _ClassVar[int]
    eid: int
    name: str
    ident: str
    coalition: _common_pb2.Coalition
    pos: _common_pb2.Vec3
    airbase_id: int
    def __init__(self, eid: _Optional[int] = ..., name: _Optional[str] = ..., ident: _Optional[str] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., pos: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ..., airbase_id: _Optional[int] = ...) -> None: ...

class WeaponSnap(_message.Message):
    __slots__ = ("eid", "type_name", "name", "pos")
    EID_FIELD_NUMBER: _ClassVar[int]
    TYPE_NAME_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    POS_FIELD_NUMBER: _ClassVar[int]
    eid: int
    type_name: str
    name: str
    pos: _common_pb2.Vec3
    def __init__(self, eid: _Optional[int] = ..., type_name: _Optional[str] = ..., name: _Optional[str] = ..., pos: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ...) -> None: ...

class WorldSnapshot(_message.Message):
    __slots__ = ("session", "rev", "model_time", "groups", "entities", "airbases")
    SESSION_FIELD_NUMBER: _ClassVar[int]
    REV_FIELD_NUMBER: _ClassVar[int]
    MODEL_TIME_FIELD_NUMBER: _ClassVar[int]
    GROUPS_FIELD_NUMBER: _ClassVar[int]
    ENTITIES_FIELD_NUMBER: _ClassVar[int]
    AIRBASES_FIELD_NUMBER: _ClassVar[int]
    session: str
    rev: int
    model_time: float
    groups: _containers.RepeatedCompositeFieldContainer[GroupSnap]
    entities: _containers.RepeatedCompositeFieldContainer[EntitySnap]
    airbases: _containers.RepeatedCompositeFieldContainer[AirbaseSnap]
    def __init__(self, session: _Optional[str] = ..., rev: _Optional[int] = ..., model_time: _Optional[float] = ..., groups: _Optional[_Iterable[_Union[GroupSnap, _Mapping]]] = ..., entities: _Optional[_Iterable[_Union[EntitySnap, _Mapping]]] = ..., airbases: _Optional[_Iterable[_Union[AirbaseSnap, _Mapping]]] = ...) -> None: ...

class WorldDelta(_message.Message):
    __slots__ = ("from_rev", "rev", "model_time", "upserts", "removals", "group_upserts", "group_removals", "airbase_upserts")
    FROM_REV_FIELD_NUMBER: _ClassVar[int]
    REV_FIELD_NUMBER: _ClassVar[int]
    MODEL_TIME_FIELD_NUMBER: _ClassVar[int]
    UPSERTS_FIELD_NUMBER: _ClassVar[int]
    REMOVALS_FIELD_NUMBER: _ClassVar[int]
    GROUP_UPSERTS_FIELD_NUMBER: _ClassVar[int]
    GROUP_REMOVALS_FIELD_NUMBER: _ClassVar[int]
    AIRBASE_UPSERTS_FIELD_NUMBER: _ClassVar[int]
    from_rev: int
    rev: int
    model_time: float
    upserts: _containers.RepeatedCompositeFieldContainer[EntitySnap]
    removals: _containers.RepeatedScalarFieldContainer[int]
    group_upserts: _containers.RepeatedCompositeFieldContainer[GroupSnap]
    group_removals: _containers.RepeatedScalarFieldContainer[int]
    airbase_upserts: _containers.RepeatedCompositeFieldContainer[AirbaseSnap]
    def __init__(self, from_rev: _Optional[int] = ..., rev: _Optional[int] = ..., model_time: _Optional[float] = ..., upserts: _Optional[_Iterable[_Union[EntitySnap, _Mapping]]] = ..., removals: _Optional[_Iterable[int]] = ..., group_upserts: _Optional[_Iterable[_Union[GroupSnap, _Mapping]]] = ..., group_removals: _Optional[_Iterable[int]] = ..., airbase_upserts: _Optional[_Iterable[_Union[AirbaseSnap, _Mapping]]] = ...) -> None: ...

class GetSnapshotRequest(_message.Message):
    __slots__ = ("session",)
    SESSION_FIELD_NUMBER: _ClassVar[int]
    session: str
    def __init__(self, session: _Optional[str] = ...) -> None: ...

class WorldQuery(_message.Message):
    __slots__ = ("coalitions", "categories", "group_names", "zones", "players_only")
    COALITIONS_FIELD_NUMBER: _ClassVar[int]
    CATEGORIES_FIELD_NUMBER: _ClassVar[int]
    GROUP_NAMES_FIELD_NUMBER: _ClassVar[int]
    ZONES_FIELD_NUMBER: _ClassVar[int]
    PLAYERS_ONLY_FIELD_NUMBER: _ClassVar[int]
    coalitions: _containers.RepeatedScalarFieldContainer[_common_pb2.Coalition]
    categories: _containers.RepeatedScalarFieldContainer[_common_pb2.Category]
    group_names: _containers.RepeatedScalarFieldContainer[str]
    zones: _containers.RepeatedScalarFieldContainer[str]
    players_only: bool
    def __init__(self, coalitions: _Optional[_Iterable[_Union[_common_pb2.Coalition, str]]] = ..., categories: _Optional[_Iterable[_Union[_common_pb2.Category, str]]] = ..., group_names: _Optional[_Iterable[str]] = ..., zones: _Optional[_Iterable[str]] = ..., players_only: _Optional[bool] = ...) -> None: ...

class SubscribeWorldRequest(_message.Message):
    __slots__ = ("session", "from_revision", "query", "field_mask")
    SESSION_FIELD_NUMBER: _ClassVar[int]
    FROM_REVISION_FIELD_NUMBER: _ClassVar[int]
    QUERY_FIELD_NUMBER: _ClassVar[int]
    FIELD_MASK_FIELD_NUMBER: _ClassVar[int]
    session: str
    from_revision: int
    query: WorldQuery
    field_mask: _field_mask_pb2.FieldMask
    def __init__(self, session: _Optional[str] = ..., from_revision: _Optional[int] = ..., query: _Optional[_Union[WorldQuery, _Mapping]] = ..., field_mask: _Optional[_Union[_field_mask_pb2.FieldMask, _Mapping]] = ...) -> None: ...
