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
    __slots__ = ("engine_version", "api", "link_major", "link_minor", "bridge_version", "dcs_version")
    ENGINE_VERSION_FIELD_NUMBER: _ClassVar[int]
    API_FIELD_NUMBER: _ClassVar[int]
    LINK_MAJOR_FIELD_NUMBER: _ClassVar[int]
    LINK_MINOR_FIELD_NUMBER: _ClassVar[int]
    BRIDGE_VERSION_FIELD_NUMBER: _ClassVar[int]
    DCS_VERSION_FIELD_NUMBER: _ClassVar[int]
    engine_version: str
    api: str
    link_major: int
    link_minor: int
    bridge_version: str
    dcs_version: str
    def __init__(self, engine_version: _Optional[str] = ..., api: _Optional[str] = ..., link_major: _Optional[int] = ..., link_minor: _Optional[int] = ..., bridge_version: _Optional[str] = ..., dcs_version: _Optional[str] = ...) -> None: ...

class HealthRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class LinkHealth(_message.Message):
    __slots__ = ("env", "connected", "credits", "events_dropped", "heartbeat_age_s")
    ENV_FIELD_NUMBER: _ClassVar[int]
    CONNECTED_FIELD_NUMBER: _ClassVar[int]
    CREDITS_FIELD_NUMBER: _ClassVar[int]
    EVENTS_DROPPED_FIELD_NUMBER: _ClassVar[int]
    HEARTBEAT_AGE_S_FIELD_NUMBER: _ClassVar[int]
    env: str
    connected: bool
    credits: int
    events_dropped: int
    heartbeat_age_s: float
    def __init__(self, env: _Optional[str] = ..., connected: _Optional[bool] = ..., credits: _Optional[int] = ..., events_dropped: _Optional[int] = ..., heartbeat_age_s: _Optional[float] = ...) -> None: ...

class HealthResponse(_message.Message):
    __slots__ = ("status", "links", "session", "scenario", "model_time", "paused", "logic_clients", "notes")
    STATUS_FIELD_NUMBER: _ClassVar[int]
    LINKS_FIELD_NUMBER: _ClassVar[int]
    SESSION_FIELD_NUMBER: _ClassVar[int]
    SCENARIO_FIELD_NUMBER: _ClassVar[int]
    MODEL_TIME_FIELD_NUMBER: _ClassVar[int]
    PAUSED_FIELD_NUMBER: _ClassVar[int]
    LOGIC_CLIENTS_FIELD_NUMBER: _ClassVar[int]
    NOTES_FIELD_NUMBER: _ClassVar[int]
    status: HealthStatus
    links: _containers.RepeatedCompositeFieldContainer[LinkHealth]
    session: str
    scenario: str
    model_time: float
    paused: bool
    logic_clients: int
    notes: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, status: _Optional[_Union[HealthStatus, str]] = ..., links: _Optional[_Iterable[_Union[LinkHealth, _Mapping]]] = ..., session: _Optional[str] = ..., scenario: _Optional[str] = ..., model_time: _Optional[float] = ..., paused: _Optional[bool] = ..., logic_clients: _Optional[int] = ..., notes: _Optional[_Iterable[str]] = ...) -> None: ...
