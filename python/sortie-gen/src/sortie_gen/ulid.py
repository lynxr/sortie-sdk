"""ULIDs (26 Crockford base32 characters: 48-bit millisecond timestamp + 80 random bits).

The engine allocates build ids with ``Generator.BeginBuild``. Offline builds use a
*content-derived* ULID instead: the timestamp part is the mission start time and the "random"
part is taken from a sha256 of the build content, so building the same scenario twice gives
the same id and byte-identical files.
"""

from __future__ import annotations

import hashlib
import os
import time
from datetime import datetime

ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
_INDEX = {c: i for i, c in enumerate(ALPHABET)}
MAX_TIMESTAMP_MS = (1 << 48) - 1


def encode(value: int) -> str:
    if not 0 <= value < (1 << 128):
        raise ValueError("a ULID is a 128-bit unsigned integer")
    chars = []
    for _ in range(26):
        chars.append(ALPHABET[value & 31])
        value >>= 5
    return "".join(reversed(chars))


def decode(text: str) -> int:
    if not is_ulid(text):
        raise ValueError(f"not a ULID: {text!r}")
    value = 0
    for c in text:
        value = (value << 5) | _INDEX[c]
    return value


def is_ulid(text: str) -> bool:
    """26 uppercase Crockford characters whose first character is 0-7 (fits 128 bits)."""
    return (
        isinstance(text, str)
        and len(text) == 26
        and text[0] in "01234567"
        and all(c in _INDEX for c in text)
    )


def from_parts(timestamp_ms: int, randomness: bytes) -> str:
    if not 0 <= timestamp_ms <= MAX_TIMESTAMP_MS:
        raise ValueError("timestamp out of the ULID range")
    if len(randomness) < 10:
        raise ValueError("a ULID needs 80 bits of randomness")
    rand = int.from_bytes(randomness[:10], "big")
    return encode((timestamp_ms << 80) | rand)


def timestamp_ms(text: str) -> int:
    return decode(text) >> 80


def new() -> str:
    """A fresh ULID from the clock and the OS random source."""
    return from_parts(int(time.time() * 1000), os.urandom(10))


def content_id(when: datetime, content: bytes) -> str:
    """The deterministic ULID of ``content`` stamped with ``when`` (timezone-aware)."""
    if when.tzinfo is None:
        raise ValueError("content_id needs a timezone-aware datetime")
    ms = int(when.timestamp() * 1000)
    return from_parts(max(0, ms), hashlib.sha256(content).digest())
