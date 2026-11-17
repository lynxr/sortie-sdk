from sortie_gen._proto.sortie.v1 import common_pb2 as _common_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class ResourceKind(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RESOURCE_KIND_UNSPECIFIED: _ClassVar[ResourceKind]
    RESOURCE_KIND_DRAWING: _ClassVar[ResourceKind]
    RESOURCE_KIND_MENU: _ClassVar[ResourceKind]
    RESOURCE_KIND_WATCH: _ClassVar[ResourceKind]
    RESOURCE_KIND_EMITTER: _ClassVar[ResourceKind]
    RESOURCE_KIND_SPOT: _ClassVar[ResourceKind]
    RESOURCE_KIND_STOCK_RULE: _ClassVar[ResourceKind]
    RESOURCE_KIND_POLICY_RULE: _ClassVar[ResourceKind]

class ResourcePhase(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RESOURCE_PHASE_UNSPECIFIED: _ClassVar[ResourcePhase]
    RESOURCE_PHASE_ACTIVE: _ClassVar[ResourcePhase]
    RESOURCE_PHASE_SUSPENDED: _ClassVar[ResourcePhase]

class ResourceChange(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RESOURCE_CHANGE_UNSPECIFIED: _ClassVar[ResourceChange]
    RESOURCE_CHANGE_CREATED: _ClassVar[ResourceChange]
    RESOURCE_CHANGE_UNCHANGED: _ClassVar[ResourceChange]
    RESOURCE_CHANGE_UPDATED: _ClassVar[ResourceChange]
    RESOURCE_CHANGE_REPLACED: _ClassVar[ResourceChange]
    RESOURCE_CHANGE_REMOVED: _ClassVar[ResourceChange]
    RESOURCE_CHANGE_ABSENT: _ClassVar[ResourceChange]

class ResourceEndReason(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    RESOURCE_END_REASON_UNSPECIFIED: _ClassVar[ResourceEndReason]
    RESOURCE_END_REASON_EXPIRED: _ClassVar[ResourceEndReason]
    RESOURCE_END_REASON_FINISHED: _ClassVar[ResourceEndReason]
    RESOURCE_END_REASON_SOURCE_GONE: _ClassVar[ResourceEndReason]
    RESOURCE_END_REASON_TARGET_GONE: _ClassVar[ResourceEndReason]
    RESOURCE_END_REASON_PLAYER: _ClassVar[ResourceEndReason]
    RESOURCE_END_REASON_LOST: _ClassVar[ResourceEndReason]
    RESOURCE_END_REASON_REMOVED_BY_OTHER: _ClassVar[ResourceEndReason]

class LineStyle(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    LINE_STYLE_UNSPECIFIED: _ClassVar[LineStyle]
    LINE_STYLE_NONE: _ClassVar[LineStyle]
    LINE_STYLE_SOLID: _ClassVar[LineStyle]
    LINE_STYLE_DASHED: _ClassVar[LineStyle]
    LINE_STYLE_DOTTED: _ClassVar[LineStyle]
    LINE_STYLE_DOT_DASH: _ClassVar[LineStyle]
    LINE_STYLE_LONG_DASH: _ClassVar[LineStyle]
    LINE_STYLE_TWO_DASH: _ClassVar[LineStyle]

class MenuOverflow(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    MENU_OVERFLOW_UNSPECIFIED: _ClassVar[MenuOverflow]
    MENU_OVERFLOW_PAGE: _ClassVar[MenuOverflow]
    MENU_OVERFLOW_REJECT: _ClassVar[MenuOverflow]
RESOURCE_KIND_UNSPECIFIED: ResourceKind
RESOURCE_KIND_DRAWING: ResourceKind
RESOURCE_KIND_MENU: ResourceKind
RESOURCE_KIND_WATCH: ResourceKind
RESOURCE_KIND_EMITTER: ResourceKind
RESOURCE_KIND_SPOT: ResourceKind
RESOURCE_KIND_STOCK_RULE: ResourceKind
RESOURCE_KIND_POLICY_RULE: ResourceKind
RESOURCE_PHASE_UNSPECIFIED: ResourcePhase
RESOURCE_PHASE_ACTIVE: ResourcePhase
RESOURCE_PHASE_SUSPENDED: ResourcePhase
RESOURCE_CHANGE_UNSPECIFIED: ResourceChange
RESOURCE_CHANGE_CREATED: ResourceChange
RESOURCE_CHANGE_UNCHANGED: ResourceChange
RESOURCE_CHANGE_UPDATED: ResourceChange
RESOURCE_CHANGE_REPLACED: ResourceChange
RESOURCE_CHANGE_REMOVED: ResourceChange
RESOURCE_CHANGE_ABSENT: ResourceChange
RESOURCE_END_REASON_UNSPECIFIED: ResourceEndReason
RESOURCE_END_REASON_EXPIRED: ResourceEndReason
RESOURCE_END_REASON_FINISHED: ResourceEndReason
RESOURCE_END_REASON_SOURCE_GONE: ResourceEndReason
RESOURCE_END_REASON_TARGET_GONE: ResourceEndReason
RESOURCE_END_REASON_PLAYER: ResourceEndReason
RESOURCE_END_REASON_LOST: ResourceEndReason
RESOURCE_END_REASON_REMOVED_BY_OTHER: ResourceEndReason
LINE_STYLE_UNSPECIFIED: LineStyle
LINE_STYLE_NONE: LineStyle
LINE_STYLE_SOLID: LineStyle
LINE_STYLE_DASHED: LineStyle
LINE_STYLE_DOTTED: LineStyle
LINE_STYLE_DOT_DASH: LineStyle
LINE_STYLE_LONG_DASH: LineStyle
LINE_STYLE_TWO_DASH: LineStyle
MENU_OVERFLOW_UNSPECIFIED: MenuOverflow
MENU_OVERFLOW_PAGE: MenuOverflow
MENU_OVERFLOW_REJECT: MenuOverflow

class ResourceSpec(_message.Message):
    __slots__ = ("draw", "menu_tree")
    DRAW_FIELD_NUMBER: _ClassVar[int]
    MENU_TREE_FIELD_NUMBER: _ClassVar[int]
    draw: Draw
    menu_tree: MenuTree
    def __init__(self, draw: _Optional[_Union[Draw, _Mapping]] = ..., menu_tree: _Optional[_Union[MenuTree, _Mapping]] = ...) -> None: ...

class ResourceSnap(_message.Message):
    __slots__ = ("kind", "name", "phase", "created_at", "updated_at", "expires_at", "owner", "spec", "updated_by", "drifted")
    KIND_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    PHASE_FIELD_NUMBER: _ClassVar[int]
    CREATED_AT_FIELD_NUMBER: _ClassVar[int]
    UPDATED_AT_FIELD_NUMBER: _ClassVar[int]
    EXPIRES_AT_FIELD_NUMBER: _ClassVar[int]
    OWNER_FIELD_NUMBER: _ClassVar[int]
    SPEC_FIELD_NUMBER: _ClassVar[int]
    UPDATED_BY_FIELD_NUMBER: _ClassVar[int]
    DRIFTED_FIELD_NUMBER: _ClassVar[int]
    kind: ResourceKind
    name: str
    phase: ResourcePhase
    created_at: float
    updated_at: float
    expires_at: float
    owner: str
    spec: ResourceSpec
    updated_by: str
    drifted: bool
    def __init__(self, kind: _Optional[_Union[ResourceKind, str]] = ..., name: _Optional[str] = ..., phase: _Optional[_Union[ResourcePhase, str]] = ..., created_at: _Optional[float] = ..., updated_at: _Optional[float] = ..., expires_at: _Optional[float] = ..., owner: _Optional[str] = ..., spec: _Optional[_Union[ResourceSpec, _Mapping]] = ..., updated_by: _Optional[str] = ..., drifted: _Optional[bool] = ...) -> None: ...

class ResourceKey(_message.Message):
    __slots__ = ("kind", "name")
    KIND_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    kind: ResourceKind
    name: str
    def __init__(self, kind: _Optional[_Union[ResourceKind, str]] = ..., name: _Optional[str] = ...) -> None: ...

class ResourcePut(_message.Message):
    __slots__ = ("name", "ttl_s", "offline_ttl_s", "draw", "menu_tree")
    NAME_FIELD_NUMBER: _ClassVar[int]
    TTL_S_FIELD_NUMBER: _ClassVar[int]
    OFFLINE_TTL_S_FIELD_NUMBER: _ClassVar[int]
    DRAW_FIELD_NUMBER: _ClassVar[int]
    MENU_TREE_FIELD_NUMBER: _ClassVar[int]
    name: str
    ttl_s: float
    offline_ttl_s: float
    draw: Draw
    menu_tree: MenuTree
    def __init__(self, name: _Optional[str] = ..., ttl_s: _Optional[float] = ..., offline_ttl_s: _Optional[float] = ..., draw: _Optional[_Union[Draw, _Mapping]] = ..., menu_tree: _Optional[_Union[MenuTree, _Mapping]] = ...) -> None: ...

class ResourceRemove(_message.Message):
    __slots__ = ("kind", "name", "prefix", "all", "any_owner")
    KIND_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    PREFIX_FIELD_NUMBER: _ClassVar[int]
    ALL_FIELD_NUMBER: _ClassVar[int]
    ANY_OWNER_FIELD_NUMBER: _ClassVar[int]
    kind: ResourceKind
    name: str
    prefix: str
    all: bool
    any_owner: bool
    def __init__(self, kind: _Optional[_Union[ResourceKind, str]] = ..., name: _Optional[str] = ..., prefix: _Optional[str] = ..., all: _Optional[bool] = ..., any_owner: _Optional[bool] = ...) -> None: ...

class ResourceAck(_message.Message):
    __slots__ = ("change", "count", "dcs_calls", "resource", "warnings", "removed")
    CHANGE_FIELD_NUMBER: _ClassVar[int]
    COUNT_FIELD_NUMBER: _ClassVar[int]
    DCS_CALLS_FIELD_NUMBER: _ClassVar[int]
    RESOURCE_FIELD_NUMBER: _ClassVar[int]
    WARNINGS_FIELD_NUMBER: _ClassVar[int]
    REMOVED_FIELD_NUMBER: _ClassVar[int]
    change: ResourceChange
    count: int
    dcs_calls: int
    resource: ResourceSnap
    warnings: _containers.RepeatedScalarFieldContainer[str]
    removed: _containers.RepeatedCompositeFieldContainer[ResourceKey]
    def __init__(self, change: _Optional[_Union[ResourceChange, str]] = ..., count: _Optional[int] = ..., dcs_calls: _Optional[int] = ..., resource: _Optional[_Union[ResourceSnap, _Mapping]] = ..., warnings: _Optional[_Iterable[str]] = ..., removed: _Optional[_Iterable[_Union[ResourceKey, _Mapping]]] = ...) -> None: ...

class ResourceEnded(_message.Message):
    __slots__ = ("kind", "name", "reason", "owner")
    KIND_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    REASON_FIELD_NUMBER: _ClassVar[int]
    OWNER_FIELD_NUMBER: _ClassVar[int]
    kind: ResourceKind
    name: str
    reason: ResourceEndReason
    owner: str
    def __init__(self, kind: _Optional[_Union[ResourceKind, str]] = ..., name: _Optional[str] = ..., reason: _Optional[_Union[ResourceEndReason, str]] = ..., owner: _Optional[str] = ...) -> None: ...

class MapStyle(_message.Message):
    __slots__ = ("color", "fill", "line")
    COLOR_FIELD_NUMBER: _ClassVar[int]
    FILL_FIELD_NUMBER: _ClassVar[int]
    LINE_FIELD_NUMBER: _ClassVar[int]
    color: _common_pb2.Rgba
    fill: _common_pb2.Rgba
    line: LineStyle
    def __init__(self, color: _Optional[_Union[_common_pb2.Rgba, _Mapping]] = ..., fill: _Optional[_Union[_common_pb2.Rgba, _Mapping]] = ..., line: _Optional[_Union[LineStyle, str]] = ...) -> None: ...

class Draw(_message.Message):
    __slots__ = ("style", "audience", "editable", "announce", "panel", "path", "circle", "rect", "arrow", "text")
    STYLE_FIELD_NUMBER: _ClassVar[int]
    AUDIENCE_FIELD_NUMBER: _ClassVar[int]
    EDITABLE_FIELD_NUMBER: _ClassVar[int]
    ANNOUNCE_FIELD_NUMBER: _ClassVar[int]
    PANEL_FIELD_NUMBER: _ClassVar[int]
    PATH_FIELD_NUMBER: _ClassVar[int]
    CIRCLE_FIELD_NUMBER: _ClassVar[int]
    RECT_FIELD_NUMBER: _ClassVar[int]
    ARROW_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    style: MapStyle
    audience: _containers.RepeatedCompositeFieldContainer[_common_pb2.Audience]
    editable: bool
    announce: str
    panel: MarkPanel
    path: MapPath
    circle: _common_pb2.MapCircle
    rect: MapRect
    arrow: MapArrow
    text: MapText
    def __init__(self, style: _Optional[_Union[MapStyle, _Mapping]] = ..., audience: _Optional[_Iterable[_Union[_common_pb2.Audience, _Mapping]]] = ..., editable: _Optional[bool] = ..., announce: _Optional[str] = ..., panel: _Optional[_Union[MarkPanel, _Mapping]] = ..., path: _Optional[_Union[MapPath, _Mapping]] = ..., circle: _Optional[_Union[_common_pb2.MapCircle, _Mapping]] = ..., rect: _Optional[_Union[MapRect, _Mapping]] = ..., arrow: _Optional[_Union[MapArrow, _Mapping]] = ..., text: _Optional[_Union[MapText, _Mapping]] = ...) -> None: ...

class MarkPanel(_message.Message):
    __slots__ = ("at", "text")
    AT_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    at: _common_pb2.Anchor
    text: str
    def __init__(self, at: _Optional[_Union[_common_pb2.Anchor, _Mapping]] = ..., text: _Optional[str] = ...) -> None: ...

class MapPath(_message.Message):
    __slots__ = ("points", "closed")
    POINTS_FIELD_NUMBER: _ClassVar[int]
    CLOSED_FIELD_NUMBER: _ClassVar[int]
    points: _containers.RepeatedCompositeFieldContainer[_common_pb2.MapPoint]
    closed: bool
    def __init__(self, points: _Optional[_Iterable[_Union[_common_pb2.MapPoint, _Mapping]]] = ..., closed: _Optional[bool] = ...) -> None: ...

class MapRect(_message.Message):
    __slots__ = ("a", "b")
    A_FIELD_NUMBER: _ClassVar[int]
    B_FIELD_NUMBER: _ClassVar[int]
    a: _common_pb2.MapPoint
    b: _common_pb2.MapPoint
    def __init__(self, a: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ..., b: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ...) -> None: ...

class MapArrow(_message.Message):
    __slots__ = ("tip", "tail")
    TIP_FIELD_NUMBER: _ClassVar[int]
    TAIL_FIELD_NUMBER: _ClassVar[int]
    tip: _common_pb2.MapPoint
    tail: _common_pb2.MapPoint
    def __init__(self, tip: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ..., tail: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ...) -> None: ...

class MapText(_message.Message):
    __slots__ = ("at", "text", "font_size")
    AT_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    FONT_SIZE_FIELD_NUMBER: _ClassVar[int]
    at: _common_pb2.MapPoint
    text: str
    font_size: int
    def __init__(self, at: _Optional[_Union[_common_pb2.MapPoint, _Mapping]] = ..., text: _Optional[str] = ..., font_size: _Optional[int] = ...) -> None: ...

class MenuTree(_message.Message):
    __slots__ = ("audience", "under", "nodes", "overflow")
    AUDIENCE_FIELD_NUMBER: _ClassVar[int]
    UNDER_FIELD_NUMBER: _ClassVar[int]
    NODES_FIELD_NUMBER: _ClassVar[int]
    OVERFLOW_FIELD_NUMBER: _ClassVar[int]
    audience: _common_pb2.Audience
    under: _containers.RepeatedScalarFieldContainer[str]
    nodes: _containers.RepeatedCompositeFieldContainer[MenuNode]
    overflow: MenuOverflow
    def __init__(self, audience: _Optional[_Union[_common_pb2.Audience, _Mapping]] = ..., under: _Optional[_Iterable[str]] = ..., nodes: _Optional[_Iterable[_Union[MenuNode, _Mapping]]] = ..., overflow: _Optional[_Union[MenuOverflow, str]] = ...) -> None: ...

class MenuNode(_message.Message):
    __slots__ = ("id", "label", "children", "ident", "arg", "keep_empty")
    ID_FIELD_NUMBER: _ClassVar[int]
    LABEL_FIELD_NUMBER: _ClassVar[int]
    CHILDREN_FIELD_NUMBER: _ClassVar[int]
    IDENT_FIELD_NUMBER: _ClassVar[int]
    ARG_FIELD_NUMBER: _ClassVar[int]
    KEEP_EMPTY_FIELD_NUMBER: _ClassVar[int]
    id: str
    label: str
    children: _containers.RepeatedCompositeFieldContainer[MenuNode]
    ident: str
    arg: str
    keep_empty: bool
    def __init__(self, id: _Optional[str] = ..., label: _Optional[str] = ..., children: _Optional[_Iterable[_Union[MenuNode, _Mapping]]] = ..., ident: _Optional[str] = ..., arg: _Optional[str] = ..., keep_empty: _Optional[bool] = ...) -> None: ...
