from sortie_gen._proto.sortie.v1 import common_pb2 as _common_pb2
from sortie_gen._proto.sortie.v1 import logic_pb2 as _logic_pb2
from sortie_gen._proto.sortie.v1 import world_pb2 as _world_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class Fate(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    FATE_UNSPECIFIED: _ClassVar[Fate]
    FATE_ALIVE: _ClassVar[Fate]
    FATE_DEAD: _ClassVar[Fate]
    FATE_GONE: _ClassVar[Fate]
    FATE_NEVER_SEEN: _ClassVar[Fate]
FATE_UNSPECIFIED: Fate
FATE_ALIVE: Fate
FATE_DEAD: Fate
FATE_GONE: Fate
FATE_NEVER_SEEN: Fate

class RegisterRequest(_message.Message):
    __slots__ = ("header", "miz_chunk")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    MIZ_CHUNK_FIELD_NUMBER: _ClassVar[int]
    header: RegisterHeader
    miz_chunk: bytes
    def __init__(self, header: _Optional[_Union[RegisterHeader, _Mapping]] = ..., miz_chunk: _Optional[bytes] = ...) -> None: ...

class RegisterHeader(_message.Message):
    __slots__ = ("build_id", "manifest_pb", "miz_sha256", "miz_size", "file_name")
    BUILD_ID_FIELD_NUMBER: _ClassVar[int]
    MANIFEST_PB_FIELD_NUMBER: _ClassVar[int]
    MIZ_SHA256_FIELD_NUMBER: _ClassVar[int]
    MIZ_SIZE_FIELD_NUMBER: _ClassVar[int]
    FILE_NAME_FIELD_NUMBER: _ClassVar[int]
    build_id: str
    manifest_pb: bytes
    miz_sha256: str
    miz_size: int
    file_name: str
    def __init__(self, build_id: _Optional[str] = ..., manifest_pb: _Optional[bytes] = ..., miz_sha256: _Optional[str] = ..., miz_size: _Optional[int] = ..., file_name: _Optional[str] = ...) -> None: ...

class RegisterResponse(_message.Message):
    __slots__ = ("build_id", "path", "digest")
    BUILD_ID_FIELD_NUMBER: _ClassVar[int]
    PATH_FIELD_NUMBER: _ClassVar[int]
    DIGEST_FIELD_NUMBER: _ClassVar[int]
    build_id: str
    path: str
    digest: str
    def __init__(self, build_id: _Optional[str] = ..., path: _Optional[str] = ..., digest: _Optional[str] = ...) -> None: ...

class LaunchRequest(_message.Message):
    __slots__ = ("build_id",)
    BUILD_ID_FIELD_NUMBER: _ClassVar[int]
    build_id: str
    def __init__(self, build_id: _Optional[str] = ...) -> None: ...

class LaunchResponse(_message.Message):
    __slots__ = ("build_id", "path")
    BUILD_ID_FIELD_NUMBER: _ClassVar[int]
    PATH_FIELD_NUMBER: _ClassVar[int]
    build_id: str
    path: str
    def __init__(self, build_id: _Optional[str] = ..., path: _Optional[str] = ...) -> None: ...

class RestartRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class RestartResponse(_message.Message):
    __slots__ = ("ended_session",)
    ENDED_SESSION_FIELD_NUMBER: _ClassVar[int]
    ended_session: str
    def __init__(self, ended_session: _Optional[str] = ...) -> None: ...

class GetResultsRequest(_message.Message):
    __slots__ = ("session",)
    SESSION_FIELD_NUMBER: _ClassVar[int]
    session: str
    def __init__(self, session: _Optional[str] = ...) -> None: ...

class EntityFate(_message.Message):
    __slots__ = ("eid", "name", "type_name", "group_name", "group_ident", "coalition", "fate", "last_pos")
    EID_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    TYPE_NAME_FIELD_NUMBER: _ClassVar[int]
    GROUP_NAME_FIELD_NUMBER: _ClassVar[int]
    GROUP_IDENT_FIELD_NUMBER: _ClassVar[int]
    COALITION_FIELD_NUMBER: _ClassVar[int]
    FATE_FIELD_NUMBER: _ClassVar[int]
    LAST_POS_FIELD_NUMBER: _ClassVar[int]
    eid: int
    name: str
    type_name: str
    group_name: str
    group_ident: str
    coalition: _common_pb2.Coalition
    fate: Fate
    last_pos: _common_pb2.Vec3
    def __init__(self, eid: _Optional[int] = ..., name: _Optional[str] = ..., type_name: _Optional[str] = ..., group_name: _Optional[str] = ..., group_ident: _Optional[str] = ..., coalition: _Optional[_Union[_common_pb2.Coalition, str]] = ..., fate: _Optional[_Union[Fate, str]] = ..., last_pos: _Optional[_Union[_common_pb2.Vec3, _Mapping]] = ...) -> None: ...

class KillRecord(_message.Message):
    __slots__ = ("model_time", "killer_eid", "killer", "killer_player", "victim_eid", "victim", "weapon")
    MODEL_TIME_FIELD_NUMBER: _ClassVar[int]
    KILLER_EID_FIELD_NUMBER: _ClassVar[int]
    KILLER_FIELD_NUMBER: _ClassVar[int]
    KILLER_PLAYER_FIELD_NUMBER: _ClassVar[int]
    VICTIM_EID_FIELD_NUMBER: _ClassVar[int]
    VICTIM_FIELD_NUMBER: _ClassVar[int]
    WEAPON_FIELD_NUMBER: _ClassVar[int]
    model_time: float
    killer_eid: int
    killer: str
    killer_player: str
    victim_eid: int
    victim: str
    weapon: str
    def __init__(self, model_time: _Optional[float] = ..., killer_eid: _Optional[int] = ..., killer: _Optional[str] = ..., killer_player: _Optional[str] = ..., victim_eid: _Optional[int] = ..., victim: _Optional[str] = ..., weapon: _Optional[str] = ...) -> None: ...

class PlayerRecord(_message.Message):
    __slots__ = ("name", "ucid", "units")
    NAME_FIELD_NUMBER: _ClassVar[int]
    UCID_FIELD_NUMBER: _ClassVar[int]
    UNITS_FIELD_NUMBER: _ClassVar[int]
    name: str
    ucid: str
    units: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, name: _Optional[str] = ..., ucid: _Optional[str] = ..., units: _Optional[_Iterable[str]] = ...) -> None: ...

class MissionResults(_message.Message):
    __slots__ = ("session", "scenario", "build_id", "outcome", "text", "end_reason", "model_time", "fates", "kills", "flags", "players", "report_json", "final_world")
    class FlagsEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: int
        def __init__(self, key: _Optional[str] = ..., value: _Optional[int] = ...) -> None: ...
    class ReportJsonEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    SESSION_FIELD_NUMBER: _ClassVar[int]
    SCENARIO_FIELD_NUMBER: _ClassVar[int]
    BUILD_ID_FIELD_NUMBER: _ClassVar[int]
    OUTCOME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    END_REASON_FIELD_NUMBER: _ClassVar[int]
    MODEL_TIME_FIELD_NUMBER: _ClassVar[int]
    FATES_FIELD_NUMBER: _ClassVar[int]
    KILLS_FIELD_NUMBER: _ClassVar[int]
    FLAGS_FIELD_NUMBER: _ClassVar[int]
    PLAYERS_FIELD_NUMBER: _ClassVar[int]
    REPORT_JSON_FIELD_NUMBER: _ClassVar[int]
    FINAL_WORLD_FIELD_NUMBER: _ClassVar[int]
    session: str
    scenario: str
    build_id: str
    outcome: _common_pb2.Outcome
    text: str
    end_reason: _logic_pb2.EndReason
    model_time: float
    fates: _containers.RepeatedCompositeFieldContainer[EntityFate]
    kills: _containers.RepeatedCompositeFieldContainer[KillRecord]
    flags: _containers.ScalarMap[str, int]
    players: _containers.RepeatedCompositeFieldContainer[PlayerRecord]
    report_json: _containers.ScalarMap[str, str]
    final_world: _world_pb2.WorldSnapshot
    def __init__(self, session: _Optional[str] = ..., scenario: _Optional[str] = ..., build_id: _Optional[str] = ..., outcome: _Optional[_Union[_common_pb2.Outcome, str]] = ..., text: _Optional[str] = ..., end_reason: _Optional[_Union[_logic_pb2.EndReason, str]] = ..., model_time: _Optional[float] = ..., fates: _Optional[_Iterable[_Union[EntityFate, _Mapping]]] = ..., kills: _Optional[_Iterable[_Union[KillRecord, _Mapping]]] = ..., flags: _Optional[_Mapping[str, int]] = ..., players: _Optional[_Iterable[_Union[PlayerRecord, _Mapping]]] = ..., report_json: _Optional[_Mapping[str, str]] = ..., final_world: _Optional[_Union[_world_pb2.WorldSnapshot, _Mapping]] = ...) -> None: ...
