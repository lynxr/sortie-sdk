from sortie_gen._proto.sortie.v1 import common_pb2 as _common_pb2
from sortie_gen._proto.sortie.v1 import missions_pb2 as _missions_pb2
from sortie_gen._proto.sortie.v1 import state_pb2 as _state_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class BeginBuildRequest(_message.Message):
    __slots__ = ("scenario", "schema", "schema_sha256", "generator_version")
    SCENARIO_FIELD_NUMBER: _ClassVar[int]
    SCHEMA_FIELD_NUMBER: _ClassVar[int]
    SCHEMA_SHA256_FIELD_NUMBER: _ClassVar[int]
    GENERATOR_VERSION_FIELD_NUMBER: _ClassVar[int]
    scenario: str
    schema: int
    schema_sha256: str
    generator_version: str
    def __init__(self, scenario: _Optional[str] = ..., schema: _Optional[int] = ..., schema_sha256: _Optional[str] = ..., generator_version: _Optional[str] = ...) -> None: ...

class BeginBuildResponse(_message.Message):
    __slots__ = ("build_id",)
    BUILD_ID_FIELD_NUMBER: _ClassVar[int]
    build_id: str
    def __init__(self, build_id: _Optional[str] = ...) -> None: ...

class ValidateTypesRequest(_message.Message):
    __slots__ = ("type_names",)
    TYPE_NAMES_FIELD_NUMBER: _ClassVar[int]
    type_names: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, type_names: _Optional[_Iterable[str]] = ...) -> None: ...

class TypeCheck(_message.Message):
    __slots__ = ("type_name", "known", "category")
    TYPE_NAME_FIELD_NUMBER: _ClassVar[int]
    KNOWN_FIELD_NUMBER: _ClassVar[int]
    CATEGORY_FIELD_NUMBER: _ClassVar[int]
    type_name: str
    known: bool
    category: _common_pb2.Category
    def __init__(self, type_name: _Optional[str] = ..., known: _Optional[bool] = ..., category: _Optional[_Union[_common_pb2.Category, str]] = ...) -> None: ...

class ValidateTypesResponse(_message.Message):
    __slots__ = ("results",)
    RESULTS_FIELD_NUMBER: _ClassVar[int]
    results: _containers.RepeatedCompositeFieldContainer[TypeCheck]
    def __init__(self, results: _Optional[_Iterable[_Union[TypeCheck, _Mapping]]] = ...) -> None: ...

class WorkerUp(_message.Message):
    __slots__ = ("hello", "response")
    HELLO_FIELD_NUMBER: _ClassVar[int]
    RESPONSE_FIELD_NUMBER: _ClassVar[int]
    hello: WorkerHello
    response: GenerateResponse
    def __init__(self, hello: _Optional[_Union[WorkerHello, _Mapping]] = ..., response: _Optional[_Union[GenerateResponse, _Mapping]] = ...) -> None: ...

class WorkerHello(_message.Message):
    __slots__ = ("scenarios", "worker", "version")
    SCENARIOS_FIELD_NUMBER: _ClassVar[int]
    WORKER_FIELD_NUMBER: _ClassVar[int]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    scenarios: _containers.RepeatedScalarFieldContainer[str]
    worker: str
    version: str
    def __init__(self, scenarios: _Optional[_Iterable[str]] = ..., worker: _Optional[str] = ..., version: _Optional[str] = ...) -> None: ...

class WorkerDown(_message.Message):
    __slots__ = ("request",)
    REQUEST_FIELD_NUMBER: _ClassVar[int]
    request: GenerateRequest
    def __init__(self, request: _Optional[_Union[GenerateRequest, _Mapping]] = ...) -> None: ...

class GenerateRequest(_message.Message):
    __slots__ = ("request_id", "scenario", "state", "results", "params")
    class ParamsEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    REQUEST_ID_FIELD_NUMBER: _ClassVar[int]
    SCENARIO_FIELD_NUMBER: _ClassVar[int]
    STATE_FIELD_NUMBER: _ClassVar[int]
    RESULTS_FIELD_NUMBER: _ClassVar[int]
    PARAMS_FIELD_NUMBER: _ClassVar[int]
    request_id: str
    scenario: str
    state: _containers.RepeatedCompositeFieldContainer[_state_pb2.StateDoc]
    results: _missions_pb2.MissionResults
    params: _containers.ScalarMap[str, str]
    def __init__(self, request_id: _Optional[str] = ..., scenario: _Optional[str] = ..., state: _Optional[_Iterable[_Union[_state_pb2.StateDoc, _Mapping]]] = ..., results: _Optional[_Union[_missions_pb2.MissionResults, _Mapping]] = ..., params: _Optional[_Mapping[str, str]] = ...) -> None: ...

class GenerateResponse(_message.Message):
    __slots__ = ("request_id", "build_id", "error")
    REQUEST_ID_FIELD_NUMBER: _ClassVar[int]
    BUILD_ID_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    request_id: str
    build_id: str
    error: GenerateError
    def __init__(self, request_id: _Optional[str] = ..., build_id: _Optional[str] = ..., error: _Optional[_Union[GenerateError, _Mapping]] = ...) -> None: ...

class GenerateError(_message.Message):
    __slots__ = ("message", "traceback")
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    TRACEBACK_FIELD_NUMBER: _ClassVar[int]
    message: str
    traceback: str
    def __init__(self, message: _Optional[str] = ..., traceback: _Optional[str] = ...) -> None: ...
