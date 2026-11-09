import datetime

from google.protobuf import timestamp_pb2 as _timestamp_pb2
from sortie_gen._proto.sortie.v1 import common_pb2 as _common_pb2
from sortie_gen._proto.sortie.v1 import world_pb2 as _world_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class EventKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    EVENT_KIND_UNSPECIFIED: _ClassVar[EventKind]
    EVENT_KIND_BIRTH: _ClassVar[EventKind]
    EVENT_KIND_DEAD: _ClassVar[EventKind]
    EVENT_KIND_UNIT_LOST: _ClassVar[EventKind]
    EVENT_KIND_KILL: _ClassVar[EventKind]
    EVENT_KIND_HIT: _ClassVar[EventKind]
    EVENT_KIND_SHOT: _ClassVar[EventKind]
    EVENT_KIND_TAKEOFF: _ClassVar[EventKind]
    EVENT_KIND_LAND: _ClassVar[EventKind]
    EVENT_KIND_BASE_CAPTURED: _ClassVar[EventKind]
    EVENT_KIND_MARK: _ClassVar[EventKind]
    EVENT_KIND_PLAYER_ENTERED_UNIT: _ClassVar[EventKind]
    EVENT_KIND_UNIT_GONE: _ClassVar[EventKind]
    EVENT_KIND_MENU_PRESSED: _ClassVar[EventKind]
    EVENT_KIND_PLAYER_CONNECTED: _ClassVar[EventKind]
    EVENT_KIND_PLAYER_DISCONNECTED: _ClassVar[EventKind]
    EVENT_KIND_PLAYER_CHANGED_SLOT: _ClassVar[EventKind]
    EVENT_KIND_CHAT_MESSAGE: _ClassVar[EventKind]
    EVENT_KIND_SIMULATION_PAUSED: _ClassVar[EventKind]
    EVENT_KIND_SIMULATION_RESUMED: _ClassVar[EventKind]
    EVENT_KIND_GAP: _ClassVar[EventKind]
    EVENT_KIND_UNKNOWN: _ClassVar[EventKind]

class MarkAction(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MARK_ACTION_UNSPECIFIED: _ClassVar[MarkAction]
    MARK_ACTION_ADDED: _ClassVar[MarkAction]
    MARK_ACTION_CHANGED: _ClassVar[MarkAction]
    MARK_ACTION_REMOVED: _ClassVar[MarkAction]

class GoneReason(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    GONE_REASON_UNSPECIFIED: _ClassVar[GoneReason]
    GONE_REASON_MISSING_FROM_SWEEP: _ClassVar[GoneReason]
    GONE_REASON_RUNTIME_ID_REUSED: _ClassVar[GoneReason]
    GONE_REASON_DESPAWNED: _ClassVar[GoneReason]
    GONE_REASON_MISSING_FROM_SNAPSHOT: _ClassVar[GoneReason]
EVENT_KIND_UNSPECIFIED: EventKind
EVENT_KIND_BIRTH: EventKind
EVENT_KIND_DEAD: EventKind
EVENT_KIND_UNIT_LOST: EventKind
EVENT_KIND_KILL: EventKind
EVENT_KIND_HIT: EventKind
EVENT_KIND_SHOT: EventKind
EVENT_KIND_TAKEOFF: EventKind
EVENT_KIND_LAND: EventKind
EVENT_KIND_BASE_CAPTURED: EventKind
EVENT_KIND_MARK: EventKind
EVENT_KIND_PLAYER_ENTERED_UNIT: EventKind
EVENT_KIND_UNIT_GONE: EventKind
EVENT_KIND_MENU_PRESSED: EventKind
EVENT_KIND_PLAYER_CONNECTED: EventKind
EVENT_KIND_PLAYER_DISCONNECTED: EventKind
EVENT_KIND_PLAYER_CHANGED_SLOT: EventKind
EVENT_KIND_CHAT_MESSAGE: EventKind
EVENT_KIND_SIMULATION_PAUSED: EventKind
EVENT_KIND_SIMULATION_RESUMED: EventKind
EVENT_KIND_GAP: EventKind
EVENT_KIND_UNKNOWN: EventKind
MARK_ACTION_UNSPECIFIED: MarkAction
MARK_ACTION_ADDED: MarkAction
MARK_ACTION_CHANGED: MarkAction
MARK_ACTION_REMOVED: MarkAction
GONE_REASON_UNSPECIFIED: GoneReason
GONE_REASON_MISSING_FROM_SWEEP: GoneReason
GONE_REASON_RUNTIME_ID_REUSED: GoneReason
GONE_REASON_DESPAWNED: GoneReason
GONE_REASON_MISSING_FROM_SNAPSHOT: GoneReason

class Event(_message.Message):
    __slots__ = ("seq", "model_time", "wall_time", "session", "birth", "dead", "unit_lost", "kill", "hit", "shot", "takeoff", "land", "base_captured", "mark", "player_entered_unit", "unit_gone", "menu_pressed", "player_connected", "player_disconnected", "player_changed_slot", "chat_message", "simulation_paused", "simulation_resumed", "gap", "unknown")
    SEQ_FIELD_NUMBER: _ClassVar[int]
    MODEL_TIME_FIELD_NUMBER: _ClassVar[int]
    WALL_TIME_FIELD_NUMBER: _ClassVar[int]
    SESSION_FIELD_NUMBER: _ClassVar[int]
    BIRTH_FIELD_NUMBER: _ClassVar[int]
    DEAD_FIELD_NUMBER: _ClassVar[int]
    UNIT_LOST_FIELD_NUMBER: _ClassVar[int]
    KILL_FIELD_NUMBER: _ClassVar[int]
    HIT_FIELD_NUMBER: _ClassVar[int]
    SHOT_FIELD_NUMBER: _ClassVar[int]
    TAKEOFF_FIELD_NUMBER: _ClassVar[int]
    LAND_FIELD_NUMBER: _ClassVar[int]
    BASE_CAPTURED_FIELD_NUMBER: _ClassVar[int]
    MARK_FIELD_NUMBER: _ClassVar[int]
    PLAYER_ENTERED_UNIT_FIELD_NUMBER: _ClassVar[int]
    UNIT_GONE_FIELD_NUMBER: _ClassVar[int]
    MENU_PRESSED_FIELD_NUMBER: _ClassVar[int]
    PLAYER_CONNECTED_FIELD_NUMBER: _ClassVar[int]
    PLAYER_DISCONNECTED_FIELD_NUMBER: _ClassVar[int]
    PLAYER_CHANGED_SLOT_FIELD_NUMBER: _ClassVar[int]
    CHAT_MESSAGE_FIELD_NUMBER: _ClassVar[int]
    SIMULATION_PAUSED_FIELD_NUMBER: _ClassVar[int]
    SIMULATION_RESUMED_FIELD_NUMBER: _ClassVar[int]
    GAP_FIELD_NUMBER: _ClassVar[int]
    UNKNOWN_FIELD_NUMBER: _ClassVar[int]
    seq: int
    model_time: float
    wall_time: _timestamp_pb2.Timestamp
    session: str
    birth: Birth
    dead: Dead
    unit_lost: UnitLost
    kill: Kill
    hit: Hit
    shot: Shot
    takeoff: Takeoff
    land: Land
    base_captured: BaseCaptured
    mark: Mark
    player_entered_unit: PlayerEnteredUnit
    unit_gone: UnitGone
    menu_pressed: MenuPressed
    player_connected: PlayerConnected
    player_disconnected: PlayerDisconnected
    player_changed_slot: PlayerChangedSlot
    chat_message: ChatMessage
    simulation_paused: SimulationPaused
    simulation_resumed: SimulationResumed
    gap: Gap
    unknown: Unknown
    def __init__(self, seq: _Optional[int] = ..., model_time: _Optional[float] = ..., wall_time: _Optional[_Union[datetime.datetime, _timestamp_pb2.Timestamp, _Mapping]] = ..., session: _Optional[str] = ..., birth: _Optional[_Union[Birth, _Mapping]] = ..., dead: _Optional[_Union[Dead, _Mapping]] = ..., unit_lost: _Optional[_Union[UnitLost, _Mapping]] = ..., kill: _Optional[_Union[Kill, _Mapping]] = ..., hit: _Optional[_Union[Hit, _Mapping]] = ..., shot: _Optional[_Union[Shot, _Mapping]] = ..., takeoff: _Optional[_Union[Takeoff, _Mapping]] = ..., land: _Optional[_Union[Land, _Mapping]] = ..., base_captured: _Optional[_Union[BaseCaptured, _Mapping]] = ..., mark: _Optional[_Union[Mark, _Mapping]] = ..., player_entered_unit: _Optional[_Union[PlayerEnteredUnit, _Mapping]] = ..., unit_gone: _Optional[_Union[UnitGone, _Mapping]] = ..., menu_pressed: _Optional[_Union[MenuPressed, _Mapping]] = ..., player_connected: _Optional[_Union[PlayerConnected, _Mapping]] = ..., player_disconnected: _Optional[_Union[PlayerDisconnected, _Mapping]] = ..., player_changed_slot: _Optional[_Union[PlayerChangedSlot, _Mapping]] = ..., chat_message: _Optional[_Union[ChatMessage, _Mapping]] = ..., simulation_paused: _Optional[_Union[SimulationPaused, _Mapping]] = ..., simulation_resumed: _Optional[_Union[SimulationResumed, _Mapping]] = ..., gap: _Optional[_Union[Gap, _Mapping]] = ..., unknown: _Optional[_Union[Unknown, _Mapping]] = ...) -> None: ...

class SubscribeEventsRequest(_message.Message):
    __slots__ = ("kinds", "coalitions", "from_seq", "session")
    KINDS_FIELD_NUMBER: _ClassVar[int]
    COALITIONS_FIELD_NUMBER: _ClassVar[int]
    FROM_SEQ_FIELD_NUMBER: _ClassVar[int]
    SESSION_FIELD_NUMBER: _ClassVar[int]
    kinds: _containers.RepeatedScalarFieldContainer[EventKind]
    coalitions: _containers.RepeatedScalarFieldContainer[_common_pb2.Coalition]
    from_seq: int
    session: str
    def __init__(self, kinds: _Optional[_Iterable[_Union[EventKind, str]]] = ..., coalitions: _Optional[_Iterable[_Union[_common_pb2.Coalition, str]]] = ..., from_seq: _Optional[int] = ..., session: _Optional[str] = ...) -> None: ...

class Birth(_message.Message):
    __slots__ = ("initiator", "place")
    INITIATOR_FIELD_NUMBER: _ClassVar[int]
    PLACE_FIELD_NUMBER: _ClassVar[int]
    initiator: _world_pb2.EntitySnap
    place: _world_pb2.AirbaseSnap
    def __init__(self, initiator: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ..., place: _Optional[_Union[_world_pb2.AirbaseSnap, _Mapping]] = ...) -> None: ...

class Dead(_message.Message):
    __slots__ = ("initiator",)
    INITIATOR_FIELD_NUMBER: _ClassVar[int]
    initiator: _world_pb2.EntitySnap
    def __init__(self, initiator: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ...) -> None: ...

class UnitLost(_message.Message):
    __slots__ = ("initiator",)
    INITIATOR_FIELD_NUMBER: _ClassVar[int]
    initiator: _world_pb2.EntitySnap
    def __init__(self, initiator: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ...) -> None: ...

class Kill(_message.Message):
    __slots__ = ("initiator", "target", "weapon")
    INITIATOR_FIELD_NUMBER: _ClassVar[int]
    TARGET_FIELD_NUMBER: _ClassVar[int]
    WEAPON_FIELD_NUMBER: _ClassVar[int]
    initiator: _world_pb2.EntitySnap
    target: _world_pb2.EntitySnap
    weapon: _world_pb2.WeaponSnap
    def __init__(self, initiator: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ..., target: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ..., weapon: _Optional[_Union[_world_pb2.WeaponSnap, _Mapping]] = ...) -> None: ...

class Hit(_message.Message):
    __slots__ = ("initiator", "target", "weapon")
    INITIATOR_FIELD_NUMBER: _ClassVar[int]
    TARGET_FIELD_NUMBER: _ClassVar[int]
    WEAPON_FIELD_NUMBER: _ClassVar[int]
    initiator: _world_pb2.EntitySnap
    target: _world_pb2.EntitySnap
    weapon: _world_pb2.WeaponSnap
    def __init__(self, initiator: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ..., target: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ..., weapon: _Optional[_Union[_world_pb2.WeaponSnap, _Mapping]] = ...) -> None: ...

class Shot(_message.Message):
    __slots__ = ("initiator", "weapon")
    INITIATOR_FIELD_NUMBER: _ClassVar[int]
    WEAPON_FIELD_NUMBER: _ClassVar[int]
    initiator: _world_pb2.EntitySnap
    weapon: _world_pb2.WeaponSnap
    def __init__(self, initiator: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ..., weapon: _Optional[_Union[_world_pb2.WeaponSnap, _Mapping]] = ...) -> None: ...

class Takeoff(_message.Message):
    __slots__ = ("initiator", "place")
    INITIATOR_FIELD_NUMBER: _ClassVar[int]
    PLACE_FIELD_NUMBER: _ClassVar[int]
    initiator: _world_pb2.EntitySnap
    place: _world_pb2.AirbaseSnap
    def __init__(self, initiator: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ..., place: _Optional[_Union[_world_pb2.AirbaseSnap, _Mapping]] = ...) -> None: ...

class Land(_message.Message):
    __slots__ = ("initiator", "place")
    INITIATOR_FIELD_NUMBER: _ClassVar[int]
    PLACE_FIELD_NUMBER: _ClassVar[int]
    initiator: _world_pb2.EntitySnap
    place: _world_pb2.AirbaseSnap
    def __init__(self, initiator: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ..., place: _Optional[_Union[_world_pb2.AirbaseSnap, _Mapping]] = ...) -> None: ...

class BaseCaptured(_message.Message):
    __slots__ = ("initiator", "place", "coalition")
    INITIATOR_FIELD_NUMBER: _ClassVar[int]
    PLACE_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    initiator: _world_pb2.EntitySnap
    place: _world_pb2.AirbaseSnap
    coalition: _common_pb2.Coalition
    def __init__(self, initiator: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ..., place: _Optional[_Union[_world_pb2.AirbaseSnap, _Mapping]] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ...) -> None: ...

class Mark(_message.Message):
    __slots__ = ("action", "mark_id", "text", "pos", "coalition", "group_eid", "initiator")
    ACTION_FIELD_NUMBER: _ClassVar[int]
    MARK_ID_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    POS_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    GROUP_EID_FIELD_NUMBER: _ClassVar[int]
    INITIATOR_FIELD_NUMBER: _ClassVar[int]
    action: MarkAction
    mark_id: int
    text: str
    pos: _common_pb2.Vec3
    coalition: _common_pb2.Coalition
    group_eid: int
    initiator: _world_pb2.EntitySnap
    def __init__(self, action: _Optional[_Union[MarkAction, str]] = ..., mark_id: _Optional[int] = ..., text: _Optional[str] = ..., pos: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., group_eid: _Optional[int] = ..., initiator: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ...) -> None: ...

class PlayerEnteredUnit(_message.Message):
    __slots__ = ("unit", "player")
    UNIT_FIELD_NUMBER: _ClassVar[int]
    PLAYER_FIELD_NUMBER: _ClassVar[int]
    unit: _world_pb2.EntitySnap
    player: str
    def __init__(self, unit: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ..., player: _Optional[str] = ...) -> None: ...

class UnitGone(_message.Message):
    __slots__ = ("eid", "name", "reason", "last_known")
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    REASON_FIELD_NUMBER: _ClassVar[int]
    LAST_KNOWN_FIELD_NUMBER: _ClassVar[int]
    eid: int
    name: str
    reason: GoneReason
    last_known: _world_pb2.EntitySnap
    def __init__(self, eid: _Optional[int] = ..., name: _Optional[str] = ..., reason: _Optional[_Union[GoneReason, str]] = ..., last_known: _Optional[_Union[_world_pb2.EntitySnap, _Mapping]] = ...) -> None: ...

class MenuPressed(_message.Message):
    __slots__ = ("ident", "group_eid", "group_name", "coalition")
    IDENT_FIELD_NUMBER: _ClassVar[int]
    GROUP_EID_FIELD_NUMBER: _ClassVar[int]
    GROUP_NAME_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    ident: str
    group_eid: int
    group_name: str
    coalition: _common_pb2.Coalition
    def __init__(self, ident: _Optional[str] = ..., group_eid: _Optional[int] = ..., group_name: _Optional[str] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ...) -> None: ...

class PlayerConnected(_message.Message):
    __slots__ = ("player_id", "name", "ucid")
    PLAYER_ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    UCID_FIELD_NUMBER: _ClassVar[int]
    player_id: int
    name: str
    ucid: str
    def __init__(self, player_id: _Optional[int] = ..., name: _Optional[str] = ..., ucid: _Optional[str] = ...) -> None: ...

class PlayerDisconnected(_message.Message):
    __slots__ = ("player_id", "name", "ucid")
    PLAYER_ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    UCID_FIELD_NUMBER: _ClassVar[int]
    player_id: int
    name: str
    ucid: str
    def __init__(self, player_id: _Optional[int] = ..., name: _Optional[str] = ..., ucid: _Optional[str] = ...) -> None: ...

class PlayerChangedSlot(_message.Message):
    __slots__ = ("player_id", "name", "coalition", "slot")
    PLAYER_ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    SLOT_FIELD_NUMBER: _ClassVar[int]
    player_id: int
    name: str
    coalition: _common_pb2.Coalition
    slot: str
    def __init__(self, player_id: _Optional[int] = ..., name: _Optional[str] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., slot: _Optional[str] = ...) -> None: ...

class ChatMessage(_message.Message):
    __slots__ = ("player_id", "name", "text", "all")
    PLAYER_ID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    ALL_FIELD_NUMBER: _ClassVar[int]
    player_id: int
    name: str
    text: str
    all: bool
    def __init__(self, player_id: _Optional[int] = ..., name: _Optional[str] = ..., text: _Optional[str] = ..., all: _Optional[bool] = ...) -> None: ...

class SimulationPaused(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class SimulationResumed(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class Gap(_message.Message):
    __slots__ = ("from_seq", "to_seq", "reason")
    FROM_SEQ_FIELD_NUMBER: _ClassVar[int]
    TO_SEQ_FIELD_NUMBER: _ClassVar[int]
    REASON_FIELD_NUMBER: _ClassVar[int]
    from_seq: int
    to_seq: int
    reason: str
    def __init__(self, from_seq: _Optional[int] = ..., to_seq: _Optional[int] = ..., reason: _Optional[str] = ...) -> None: ...

class Unknown(_message.Message):
    __slots__ = ("dcs_name", "dcs_id", "fields")
    DCS_NAME_FIELD_NUMBER: _ClassVar[int]
    DCS_ID_FIELD_NUMBER: _ClassVar[int]
    FIELDS_FIELD_NUMBER: _ClassVar[int]
    dcs_name: str
    dcs_id: int
    fields: _common_pb2.LuaValue
    def __init__(self, dcs_name: _Optional[str] = ..., dcs_id: _Optional[int] = ..., fields: _Optional[_Union[_common_pb2.LuaValue, _Mapping]] = ...) -> None: ...
