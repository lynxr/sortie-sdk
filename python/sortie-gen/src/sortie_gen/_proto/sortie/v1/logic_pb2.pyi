from sortie_gen._proto.sortie.manifest.v1 import manifest_pb2 as _manifest_pb2
from sortie_gen._proto.sortie.v1 import commands_pb2 as _commands_pb2
from sortie_gen._proto.sortie.v1 import common_pb2 as _common_pb2
from sortie_gen._proto.sortie.v1 import events_pb2 as _events_pb2
from sortie_gen._proto.sortie.v1 import state_pb2 as _state_pb2
from sortie_gen._proto.sortie.v1 import world_pb2 as _world_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class Mode(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MODE_UNSPECIFIED: _ClassVar[Mode]
    MODE_PRIMARY: _ClassVar[Mode]
    MODE_STANDBY: _ClassVar[Mode]
    MODE_OBSERVER: _ClassVar[Mode]

class StartKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    START_KIND_UNSPECIFIED: _ClassVar[StartKind]
    START_KIND_FRESH: _ClassVar[StartKind]
    START_KIND_RESTART: _ClassVar[StartKind]
    START_KIND_AFTER_CRASH: _ClassVar[StartKind]
    START_KIND_RESUMED: _ClassVar[StartKind]

class EndReason(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    END_REASON_UNSPECIFIED: _ClassVar[EndReason]
    END_REASON_LOGIC: _ClassVar[EndReason]
    END_REASON_TIME_LIMIT: _ClassVar[EndReason]
    END_REASON_RESTART: _ClassVar[EndReason]
    END_REASON_MISSION_UNLOADED: _ClassVar[EndReason]
    END_REASON_ENGINE_SHUTDOWN: _ClassVar[EndReason]
MODE_UNSPECIFIED: Mode
MODE_PRIMARY: Mode
MODE_STANDBY: Mode
MODE_OBSERVER: Mode
START_KIND_UNSPECIFIED: StartKind
START_KIND_FRESH: StartKind
START_KIND_RESTART: StartKind
START_KIND_AFTER_CRASH: StartKind
START_KIND_RESUMED: StartKind
END_REASON_UNSPECIFIED: EndReason
END_REASON_LOGIC: EndReason
END_REASON_TIME_LIMIT: EndReason
END_REASON_RESTART: EndReason
END_REASON_MISSION_UNLOADED: EndReason
END_REASON_ENGINE_SHUTDOWN: EndReason

class LogicUp(_message.Message):
    __slots__ = ("attach", "done", "command", "state", "resync")
    ATTACH_FIELD_NUMBER: _ClassVar[int]
    DONE_FIELD_NUMBER: _ClassVar[int]
    COMMAND_FIELD_NUMBER: _ClassVar[int]
    STATE_FIELD_NUMBER: _ClassVar[int]
    RESYNC_FIELD_NUMBER: _ClassVar[int]
    attach: Attach
    done: TickDone
    command: _commands_pb2.Command
    state: StateOp
    resync: Resync
    def __init__(self, attach: _Optional[_Union[Attach, _Mapping]] = ..., done: _Optional[_Union[TickDone, _Mapping]] = ..., command: _Optional[_Union[_commands_pb2.Command, _Mapping]] = ..., state: _Optional[_Union[StateOp, _Mapping]] = ..., resync: _Optional[_Union[Resync, _Mapping]] = ...) -> None: ...

class Resync(_message.Message):
    __slots__ = ("have_rev", "reason")
    HAVE_REV_FIELD_NUMBER: _ClassVar[int]
    REASON_FIELD_NUMBER: _ClassVar[int]
    have_rev: int
    reason: str
    def __init__(self, have_rev: _Optional[int] = ..., reason: _Optional[str] = ...) -> None: ...

class Attach(_message.Message):
    __slots__ = ("logic", "build_id", "scenario_schema_sha256", "symbols_used", "mode", "scenario", "schema")
    LOGIC_FIELD_NUMBER: _ClassVar[int]
    BUILD_ID_FIELD_NUMBER: _ClassVar[int]
    SCENARIO_SCHEMA_SHA256_FIELD_NUMBER: _ClassVar[int]
    SYMBOLS_USED_FIELD_NUMBER: _ClassVar[int]
    MODE_FIELD_NUMBER: _ClassVar[int]
    SCENARIO_FIELD_NUMBER: _ClassVar[int]
    SCHEMA_FIELD_NUMBER: _ClassVar[int]
    logic: str
    build_id: str
    scenario_schema_sha256: bytes
    symbols_used: _containers.RepeatedScalarFieldContainer[str]
    mode: Mode
    scenario: str
    schema: int
    def __init__(self, logic: _Optional[str] = ..., build_id: _Optional[str] = ..., scenario_schema_sha256: _Optional[bytes] = ..., symbols_used: _Optional[_Iterable[str]] = ..., mode: _Optional[_Union[Mode, str]] = ..., scenario: _Optional[str] = ..., schema: _Optional[int] = ...) -> None: ...

class LogicDown(_message.Message):
    __slots__ = ("start", "tick", "event", "result", "promote", "shutdown", "ended", "state", "rejected", "resync")
    START_FIELD_NUMBER: _ClassVar[int]
    TICK_FIELD_NUMBER: _ClassVar[int]
    EVENT_FIELD_NUMBER: _ClassVar[int]
    RESULT_FIELD_NUMBER: _ClassVar[int]
    PROMOTE_FIELD_NUMBER: _ClassVar[int]
    SHUTDOWN_FIELD_NUMBER: _ClassVar[int]
    ENDED_FIELD_NUMBER: _ClassVar[int]
    STATE_FIELD_NUMBER: _ClassVar[int]
    REJECTED_FIELD_NUMBER: _ClassVar[int]
    RESYNC_FIELD_NUMBER: _ClassVar[int]
    start: Start
    tick: Tick
    event: _events_pb2.Event
    result: _commands_pb2.CommandResult
    promote: Promote
    shutdown: Shutdown
    ended: SessionEnded
    state: StateResult
    rejected: AttachRejected
    resync: _world_pb2.WorldSnapshot
    def __init__(self, start: _Optional[_Union[Start, _Mapping]] = ..., tick: _Optional[_Union[Tick, _Mapping]] = ..., event: _Optional[_Union[_events_pb2.Event, _Mapping]] = ..., result: _Optional[_Union[_commands_pb2.CommandResult, _Mapping]] = ..., promote: _Optional[_Union[Promote, _Mapping]] = ..., shutdown: _Optional[_Union[Shutdown, _Mapping]] = ..., ended: _Optional[_Union[SessionEnded, _Mapping]] = ..., state: _Optional[_Union[StateResult, _Mapping]] = ..., rejected: _Optional[_Union[AttachRejected, _Mapping]] = ..., resync: _Optional[_Union[_world_pb2.WorldSnapshot, _Mapping]] = ...) -> None: ...

class Start(_message.Message):
    __slots__ = ("session", "kind", "manifest", "snapshot", "turn", "model_time")
    SESSION_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    MANIFEST_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_FIELD_NUMBER: _ClassVar[int]
    TURN_FIELD_NUMBER: _ClassVar[int]
    MODEL_TIME_FIELD_NUMBER: _ClassVar[int]
    session: str
    kind: StartKind
    manifest: _manifest_pb2.Manifest
    snapshot: _world_pb2.WorldSnapshot
    turn: str
    model_time: float
    def __init__(self, session: _Optional[str] = ..., kind: _Optional[_Union[StartKind, str]] = ..., manifest: _Optional[_Union[_manifest_pb2.Manifest, _Mapping]] = ..., snapshot: _Optional[_Union[_world_pb2.WorldSnapshot, _Mapping]] = ..., turn: _Optional[str] = ..., model_time: _Optional[float] = ...) -> None: ...

class Tick(_message.Message):
    __slots__ = ("n", "model_time", "delta", "paused")
    N_FIELD_NUMBER: _ClassVar[int]
    MODEL_TIME_FIELD_NUMBER: _ClassVar[int]
    DELTA_FIELD_NUMBER: _ClassVar[int]
    PAUSED_FIELD_NUMBER: _ClassVar[int]
    n: int
    model_time: float
    delta: _world_pb2.WorldDelta
    paused: bool
    def __init__(self, n: _Optional[int] = ..., model_time: _Optional[float] = ..., delta: _Optional[_Union[_world_pb2.WorldDelta, _Mapping]] = ..., paused: _Optional[bool] = ...) -> None: ...

class TickDone(_message.Message):
    __slots__ = ("n",)
    N_FIELD_NUMBER: _ClassVar[int]
    n: int
    def __init__(self, n: _Optional[int] = ...) -> None: ...

class Promote(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class Shutdown(_message.Message):
    __slots__ = ("reason",)
    REASON_FIELD_NUMBER: _ClassVar[int]
    reason: str
    def __init__(self, reason: _Optional[str] = ...) -> None: ...

class SessionEnded(_message.Message):
    __slots__ = ("session", "outcome", "text", "reason")
    SESSION_FIELD_NUMBER: _ClassVar[int]
    OUTCOME_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    REASON_FIELD_NUMBER: _ClassVar[int]
    session: str
    outcome: _common_pb2.Outcome
    text: str
    reason: EndReason
    def __init__(self, session: _Optional[str] = ..., outcome: _Optional[_Union[_common_pb2.Outcome, str]] = ..., text: _Optional[str] = ..., reason: _Optional[_Union[EndReason, str]] = ...) -> None: ...

class AttachRejected(_message.Message):
    __slots__ = ("code", "message", "missing_symbols")
    CODE_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    MISSING_SYMBOLS_FIELD_NUMBER: _ClassVar[int]
    code: _common_pb2.ErrorCode
    message: str
    missing_symbols: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, code: _Optional[_Union[_common_pb2.ErrorCode, str]] = ..., message: _Optional[str] = ..., missing_symbols: _Optional[_Iterable[str]] = ...) -> None: ...

class StateOp(_message.Message):
    __slots__ = ("id", "get", "put", "once_check", "once_mark", "timer_arm", "report_set")
    ID_FIELD_NUMBER: _ClassVar[int]
    GET_FIELD_NUMBER: _ClassVar[int]
    PUT_FIELD_NUMBER: _ClassVar[int]
    ONCE_CHECK_FIELD_NUMBER: _ClassVar[int]
    ONCE_MARK_FIELD_NUMBER: _ClassVar[int]
    TIMER_ARM_FIELD_NUMBER: _ClassVar[int]
    REPORT_SET_FIELD_NUMBER: _ClassVar[int]
    id: int
    get: StateGet
    put: StatePut
    once_check: OnceCheck
    once_mark: OnceMark
    timer_arm: TimerArm
    report_set: ReportSet
    def __init__(self, id: _Optional[int] = ..., get: _Optional[_Union[StateGet, _Mapping]] = ..., put: _Optional[_Union[StatePut, _Mapping]] = ..., once_check: _Optional[_Union[OnceCheck, _Mapping]] = ..., once_mark: _Optional[_Union[OnceMark, _Mapping]] = ..., timer_arm: _Optional[_Union[TimerArm, _Mapping]] = ..., report_set: _Optional[_Union[ReportSet, _Mapping]] = ...) -> None: ...

class StateGet(_message.Message):
    __slots__ = ("scope", "name")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    scope: _state_pb2.StateScope
    name: str
    def __init__(self, scope: _Optional[_Union[_state_pb2.StateScope, str]] = ..., name: _Optional[str] = ...) -> None: ...

class StatePut(_message.Message):
    __slots__ = ("scope", "name", "expected_revision", "json")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    EXPECTED_REVISION_FIELD_NUMBER: _ClassVar[int]
    JSON_FIELD_NUMBER: _ClassVar[int]
    scope: _state_pb2.StateScope
    name: str
    expected_revision: int
    json: bytes
    def __init__(self, scope: _Optional[_Union[_state_pb2.StateScope, str]] = ..., name: _Optional[str] = ..., expected_revision: _Optional[int] = ..., json: _Optional[bytes] = ...) -> None: ...

class OnceCheck(_message.Message):
    __slots__ = ("scope", "name")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    scope: _state_pb2.StateScope
    name: str
    def __init__(self, scope: _Optional[_Union[_state_pb2.StateScope, str]] = ..., name: _Optional[str] = ...) -> None: ...

class OnceMark(_message.Message):
    __slots__ = ("scope", "name")
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    scope: _state_pb2.StateScope
    name: str
    def __init__(self, scope: _Optional[_Union[_state_pb2.StateScope, str]] = ..., name: _Optional[str] = ...) -> None: ...

class TimerArm(_message.Message):
    __slots__ = ("name", "duration_s")
    NAME_FIELD_NUMBER: _ClassVar[int]
    DURATION_S_FIELD_NUMBER: _ClassVar[int]
    name: str
    duration_s: float
    def __init__(self, name: _Optional[str] = ..., duration_s: _Optional[float] = ...) -> None: ...

class ReportSet(_message.Message):
    __slots__ = ("key", "value_json")
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_JSON_FIELD_NUMBER: _ClassVar[int]
    key: str
    value_json: str
    def __init__(self, key: _Optional[str] = ..., value_json: _Optional[str] = ...) -> None: ...

class StateResult(_message.Message):
    __slots__ = ("id", "doc", "once", "timer", "ack", "err")
    ID_FIELD_NUMBER: _ClassVar[int]
    DOC_FIELD_NUMBER: _ClassVar[int]
    ONCE_FIELD_NUMBER: _ClassVar[int]
    TIMER_FIELD_NUMBER: _ClassVar[int]
    ACK_FIELD_NUMBER: _ClassVar[int]
    ERR_FIELD_NUMBER: _ClassVar[int]
    id: int
    doc: _state_pb2.StateDoc
    once: OnceStatus
    timer: TimerStatus
    ack: StateAck
    err: _common_pb2.CommandError
    def __init__(self, id: _Optional[int] = ..., doc: _Optional[_Union[_state_pb2.StateDoc, _Mapping]] = ..., once: _Optional[_Union[OnceStatus, _Mapping]] = ..., timer: _Optional[_Union[TimerStatus, _Mapping]] = ..., ack: _Optional[_Union[StateAck, _Mapping]] = ..., err: _Optional[_Union[_common_pb2.CommandError, _Mapping]] = ...) -> None: ...

class OnceStatus(_message.Message):
    __slots__ = ("done",)
    DONE_FIELD_NUMBER: _ClassVar[int]
    done: bool
    def __init__(self, done: _Optional[bool] = ...) -> None: ...

class TimerStatus(_message.Message):
    __slots__ = ("deadline",)
    DEADLINE_FIELD_NUMBER: _ClassVar[int]
    deadline: float
    def __init__(self, deadline: _Optional[float] = ...) -> None: ...

class StateAck(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...
