from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class StateScope(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    STATE_SCOPE_UNSPECIFIED: _ClassVar[StateScope]
    STATE_SCOPE_SESSION: _ClassVar[StateScope]
    STATE_SCOPE_TURN: _ClassVar[StateScope]
    STATE_SCOPE_CAMPAIGN: _ClassVar[StateScope]
STATE_SCOPE_UNSPECIFIED: StateScope
STATE_SCOPE_SESSION: StateScope
STATE_SCOPE_TURN: StateScope
STATE_SCOPE_CAMPAIGN: StateScope

class StateDoc(_message.Message):
    __slots__ = ("scenario", "scope", "name", "revision", "json", "scope_key")
    SCENARIO_FIELD_NUMBER: _ClassVar[int]
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    REVISION_FIELD_NUMBER: _ClassVar[int]
    JSON_FIELD_NUMBER: _ClassVar[int]
    SCOPE_KEY_FIELD_NUMBER: _ClassVar[int]
    scenario: str
    scope: StateScope
    name: str
    revision: int
    json: bytes
    scope_key: str
    def __init__(self, scenario: _Optional[str] = ..., scope: _Optional[_Union[StateScope, str]] = ..., name: _Optional[str] = ..., revision: _Optional[int] = ..., json: _Optional[bytes] = ..., scope_key: _Optional[str] = ...) -> None: ...

class GetStateRequest(_message.Message):
    __slots__ = ("scenario", "scope", "name", "scope_key")
    SCENARIO_FIELD_NUMBER: _ClassVar[int]
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SCOPE_KEY_FIELD_NUMBER: _ClassVar[int]
    scenario: str
    scope: StateScope
    name: str
    scope_key: str
    def __init__(self, scenario: _Optional[str] = ..., scope: _Optional[_Union[StateScope, str]] = ..., name: _Optional[str] = ..., scope_key: _Optional[str] = ...) -> None: ...

class PutStateRequest(_message.Message):
    __slots__ = ("scenario", "scope", "name", "scope_key", "expected_revision", "json")
    SCENARIO_FIELD_NUMBER: _ClassVar[int]
    SCOPE_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    SCOPE_KEY_FIELD_NUMBER: _ClassVar[int]
    EXPECTED_REVISION_FIELD_NUMBER: _ClassVar[int]
    JSON_FIELD_NUMBER: _ClassVar[int]
    scenario: str
    scope: StateScope
    name: str
    scope_key: str
    expected_revision: int
    json: bytes
    def __init__(self, scenario: _Optional[str] = ..., scope: _Optional[_Union[StateScope, str]] = ..., name: _Optional[str] = ..., scope_key: _Optional[str] = ..., expected_revision: _Optional[int] = ..., json: _Optional[bytes] = ...) -> None: ...

class PutStateResponse(_message.Message):
    __slots__ = ("revision",)
    REVISION_FIELD_NUMBER: _ClassVar[int]
    revision: int
    def __init__(self, revision: _Optional[int] = ...) -> None: ...
