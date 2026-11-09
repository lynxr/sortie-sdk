"""``manifest.pb`` / ``manifest.json`` helpers (``sortie.manifest.v1.Manifest``).

* ``manifest.pb`` is ``Manifest.SerializeToString(deterministic=True)``; the bootstrap digest
  is the sha256 of exactly these bytes, so they are never re-encoded.
* ``manifest.json`` is ``MessageToJson(m, preserving_proto_field_name=True, indent=2) + "\\n"``
  with map entries sorted by key (the same text ``sortie_manifest::manifest_to_json`` writes).
  It is for humans; the engine and the SDK read the ``.pb``.
"""

from __future__ import annotations

import hashlib
from pathlib import Path

from google.protobuf import json_format

from ._proto.sortie.manifest.v1 import manifest_pb2 as mpb

Manifest = mpb.Manifest


def encode(m: mpb.Manifest) -> bytes:
    return m.SerializeToString(deterministic=True)


def decode(data: bytes) -> mpb.Manifest:
    m = mpb.Manifest()
    m.ParseFromString(data)
    return m


def digest(manifest_pb: bytes) -> str:
    """Lowercase hex sha256 of the exact ``manifest.pb`` bytes (the bootstrap digest)."""
    return hashlib.sha256(manifest_pb).hexdigest()


def _sorted_maps(m: mpb.Manifest) -> mpb.Manifest:
    """A copy whose map fields were filled in key order (JSON follows insertion order)."""
    out = mpb.Manifest()
    out.CopyFrom(m)
    extra = dict(out.tools.extra)
    out.tools.extra.clear()
    for k in sorted(extra):
        out.tools.extra[k] = extra[k]
    return out


def manifest_json(m: mpb.Manifest) -> str:
    return json_format.MessageToJson(_sorted_maps(m), preserving_proto_field_name=True, indent=2) + "\n"


def load(path: str | Path) -> mpb.Manifest:
    """Reads ``*.manifest.pb`` or ``*.manifest.json``."""
    p = Path(path)
    if p.suffix == ".json":
        m = mpb.Manifest()
        json_format.Parse(p.read_text(encoding="utf-8"), m)
        return m
    return decode(p.read_bytes())
