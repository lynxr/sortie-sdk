from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Optional as _Optional

DESCRIPTOR: _descriptor.FileDescriptor

class ServerStatusRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class ServerStatus(_message.Message):
    __slots__ = ("dcs_version", "hooks_connected", "mission_connected", "mission_name", "session", "paused", "model_time", "players")
    DCS_VERSION_FIELD_NUMBER: _ClassVar[int]
    HOOKS_CONNECTED_FIELD_NUMBER: _ClassVar[int]
    MISSION_CONNECTED_FIELD_NUMBER: _ClassVar[int]
    MISSION_NAME_FIELD_NUMBER: _ClassVar[int]
    SESSION_FIELD_NUMBER: _ClassVar[int]
    PAUSED_FIELD_NUMBER: _ClassVar[int]
    MODEL_TIME_FIELD_NUMBER: _ClassVar[int]
    PLAYERS_FIELD_NUMBER: _ClassVar[int]
    dcs_version: str
    hooks_connected: bool
    mission_connected: bool
    mission_name: str
    session: str
    paused: bool
    model_time: float
    players: int
    def __init__(self, dcs_version: _Optional[str] = ..., hooks_connected: _Optional[bool] = ..., mission_connected: _Optional[bool] = ..., mission_name: _Optional[str] = ..., session: _Optional[str] = ..., paused: _Optional[bool] = ..., model_time: _Optional[float] = ..., players: _Optional[int] = ...) -> None: ...

class SetPausedRequest(_message.Message):
    __slots__ = ("paused",)
    PAUSED_FIELD_NUMBER: _ClassVar[int]
    paused: bool
    def __init__(self, paused: _Optional[bool] = ...) -> None: ...

class SetPausedResponse(_message.Message):
    __slots__ = ("paused",)
    PAUSED_FIELD_NUMBER: _ClassVar[int]
    paused: bool
    def __init__(self, paused: _Optional[bool] = ...) -> None: ...
