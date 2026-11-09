"""Atomic file writes: a temporary file in the target folder, fsync, then rename over the target.

A reader (DCS, the engine, ``build.rs``) therefore sees either the old file or the new one,
never a half-written one. On Windows a rename onto a file that another process holds open
fails for a moment; the write is retried briefly before giving up with a clear message.
"""

from __future__ import annotations

import os
import secrets
import time
from pathlib import Path

from .errors import GeneratorError

_RETRIES = 20
_RETRY_DELAY_S = 0.05


def atomic_write_bytes(path: str | os.PathLike[str], data: bytes) -> Path:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    tmp = target.with_name(f".{target.name}.{os.getpid()}-{secrets.token_hex(4)}.tmp")
    try:
        with open(tmp, "wb") as f:
            f.write(data)
            f.flush()
            os.fsync(f.fileno())
        for attempt in range(_RETRIES):
            try:
                os.replace(tmp, target)
                break
            except PermissionError:
                if attempt == _RETRIES - 1:
                    raise GeneratorError(
                        f"cannot replace {target}: another program keeps it open (close DCS or the editor and retry)"
                    ) from None
                time.sleep(_RETRY_DELAY_S)
    finally:
        if tmp.exists():
            tmp.unlink()
    return target


def atomic_write_text(path: str | os.PathLike[str], text: str) -> Path:
    """UTF-8, ``\\n`` line endings on every OS (the files are compared byte for byte)."""
    return atomic_write_bytes(path, text.encode("utf-8"))


def write_if_changed(path: str | os.PathLike[str], data: bytes) -> bool:
    """Atomically writes ``data`` unless the file already holds exactly these bytes."""
    target = Path(path)
    try:
        if target.read_bytes() == data:
            return False
    except FileNotFoundError:
        pass
    atomic_write_bytes(target, data)
    return True
