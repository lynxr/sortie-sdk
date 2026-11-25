from sortie_gen._proto.sortie.v1 import common_pb2 as _common_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class HealthStatus(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    HEALTH_STATUS_UNSPECIFIED: _ClassVar[HealthStatus]
    HEALTH_STATUS_OK: _ClassVar[HealthStatus]
    HEALTH_STATUS_DEGRADED: _ClassVar[HealthStatus]
    HEALTH_STATUS_DOWN: _ClassVar[HealthStatus]
HEALTH_STATUS_UNSPECIFIED: HealthStatus
HEALTH_STATUS_OK: HealthStatus
HEALTH_STATUS_DEGRADED: HealthStatus
HEALTH_STATUS_DOWN: HealthStatus

class VersionRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class VersionResponse(_message.Message):
    __slots__ = ("engine_version", "api", "link_major", "link_minor", "bridge_version", "dcs_version", "api_rev")
    ENGINE_VERSION_FIELD_NUMBER: _ClassVar[int]
    API_FIELD_NUMBER: _ClassVar[int]
    LINK_MAJOR_FIELD_NUMBER: _ClassVar[int]
    LINK_MINOR_FIELD_NUMBER: _ClassVar[int]
    BRIDGE_VERSION_FIELD_NUMBER: _ClassVar[int]
    DCS_VERSION_FIELD_NUMBER: _ClassVar[int]
    API_REV_FIELD_NUMBER: _ClassVar[int]
    engine_version: str
    api: str
    link_major: int
    link_minor: int
    bridge_version: str
    dcs_version: str
    api_rev: int
    def __init__(self, engine_version: _Optional[str] = ..., api: _Optional[str] = ..., link_major: _Optional[int] = ..., link_minor: _Optional[int] = ..., bridge_version: _Optional[str] = ..., dcs_version: _Optional[str] = ..., api_rev: _Optional[int] = ...) -> None: ...

class HealthRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class LinkHealth(_message.Message):
    __slots__ = ("env", "connected", "credits", "events_dropped", "heartbeat_age_s", "fps", "pump_p99_us", "event_times_ignored", "event_copies", "event_copy_calls", "event_copy_us", "event_copy_max_us", "snapshot_objects", "snapshot_calls", "snapshot_us")
    ENV_FIELD_NUMBER: _ClassVar[int]
    CONNECTED_FIELD_NUMBER: _ClassVar[int]
    CREDITS_FIELD_NUMBER: _ClassVar[int]
    EVENTS_DROPPED_FIELD_NUMBER: _ClassVar[int]
    HEARTBEAT_AGE_S_FIELD_NUMBER: _ClassVar[int]
    FPS_FIELD_NUMBER: _ClassVar[int]
    PUMP_P99_US_FIELD_NUMBER: _ClassVar[int]
    EVENT_TIMES_IGNORED_FIELD_NUMBER: _ClassVar[int]
    EVENT_COPIES_FIELD_NUMBER: _ClassVar[int]
    EVENT_COPY_CALLS_FIELD_NUMBER: _ClassVar[int]
    EVENT_COPY_US_FIELD_NUMBER: _ClassVar[int]
    EVENT_COPY_MAX_US_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_OBJECTS_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_CALLS_FIELD_NUMBER: _ClassVar[int]
    SNAPSHOT_US_FIELD_NUMBER: _ClassVar[int]
    env: str
    connected: bool
    credits: int
    events_dropped: int
    heartbeat_age_s: float
    fps: float
    pump_p99_us: int
    event_times_ignored: int
    event_copies: int
    event_copy_calls: int
    event_copy_us: int
    event_copy_max_us: int
    snapshot_objects: int
    snapshot_calls: int
    snapshot_us: int
    def __init__(self, env: _Optional[str] = ..., connected: _Optional[bool] = ..., credits: _Optional[int] = ..., events_dropped: _Optional[int] = ..., heartbeat_age_s: _Optional[float] = ..., fps: _Optional[float] = ..., pump_p99_us: _Optional[int] = ..., event_times_ignored: _Optional[int] = ..., event_copies: _Optional[int] = ..., event_copy_calls: _Optional[int] = ..., event_copy_us: _Optional[int] = ..., event_copy_max_us: _Optional[int] = ..., snapshot_objects: _Optional[int] = ..., snapshot_calls: _Optional[int] = ..., snapshot_us: _Optional[int] = ...) -> None: ...

class HealthResponse(_message.Message):
    __slots__ = ("status", "links", "session", "scenario", "model_time", "paused", "logic_clients", "notes", "carried_over_quarantines")
    STATUS_FIELD_NUMBER: _ClassVar[int]
    LINKS_FIELD_NUMBER: _ClassVar[int]
    SESSION_FIELD_NUMBER: _ClassVar[int]
    SCENARIO_FIELD_NUMBER: _ClassVar[int]
    MODEL_TIME_FIELD_NUMBER: _ClassVar[int]
    PAUSED_FIELD_NUMBER: _ClassVar[int]
    LOGIC_CLIENTS_FIELD_NUMBER: _ClassVar[int]
    NOTES_FIELD_NUMBER: _ClassVar[int]
    CARRIED_OVER_QUARANTINES_FIELD_NUMBER: _ClassVar[int]
    status: HealthStatus
    links: _containers.RepeatedCompositeFieldContainer[LinkHealth]
    session: str
    scenario: str
    model_time: float
    paused: bool
    logic_clients: int
    notes: _containers.RepeatedScalarFieldContainer[str]
    carried_over_quarantines: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, status: _Optional[_Union[HealthStatus, str]] = ..., links: _Optional[_Iterable[_Union[LinkHealth, _Mapping]]] = ..., session: _Optional[str] = ..., scenario: _Optional[str] = ..., model_time: _Optional[float] = ..., paused: _Optional[bool] = ..., logic_clients: _Optional[int] = ..., notes: _Optional[_Iterable[str]] = ..., carried_over_quarantines: _Optional[_Iterable[str]] = ...) -> None: ...

class CapabilitiesRequest(_message.Message):
    __slots__ = ("prefix",)
    PREFIX_FIELD_NUMBER: _ClassVar[int]
    prefix: str
    def __init__(self, prefix: _Optional[str] = ...) -> None: ...

class CapabilitiesResponse(_message.Message):
    __slots__ = ("capabilities", "api_rev", "event_rev", "dcs_version")
    CAPABILITIES_FIELD_NUMBER: _ClassVar[int]
    API_REV_FIELD_NUMBER: _ClassVar[int]
    EVENT_REV_FIELD_NUMBER: _ClassVar[int]
    DCS_VERSION_FIELD_NUMBER: _ClassVar[int]
    capabilities: _containers.RepeatedCompositeFieldContainer[_common_pb2.Capability]
    api_rev: int
    event_rev: int
    dcs_version: str
    def __init__(self, capabilities: _Optional[_Iterable[_Union[_common_pb2.Capability, _Mapping]]] = ..., api_rev: _Optional[int] = ..., event_rev: _Optional[int] = ..., dcs_version: _Optional[str] = ...) -> None: ...

class ReleaseQuarantineRequest(_message.Message):
    __slots__ = ("name", "carried_over")
    NAME_FIELD_NUMBER: _ClassVar[int]
    CARRIED_OVER_FIELD_NUMBER: _ClassVar[int]
    name: str
    carried_over: bool
    def __init__(self, name: _Optional[str] = ..., carried_over: _Optional[bool] = ...) -> None: ...

class ReleaseQuarantineResponse(_message.Message):
    __slots__ = ("released", "capabilities")
    RELEASED_FIELD_NUMBER: _ClassVar[int]
    CAPABILITIES_FIELD_NUMBER: _ClassVar[int]
    released: _containers.RepeatedScalarFieldContainer[str]
    capabilities: _containers.RepeatedCompositeFieldContainer[_common_pb2.Capability]
    def __init__(self, released: _Optional[_Iterable[str]] = ..., capabilities: _Optional[_Iterable[_Union[_common_pb2.Capability, _Mapping]]] = ...) -> None: ...
