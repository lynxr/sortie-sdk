"""The lock file, ULIDs and the JSON shape of pydcs tables (no pydcs mission needed)."""

from __future__ import annotations

import json
from datetime import datetime, timezone

import pytest

from sortie_gen import _pydcs, lock as lockmod, ulid
from sortie_gen.errors import InvalidName, LockError

from conftest import OP_NORTH, OP_NORTH_SCHEMA_SHA256


def op_north_symbols() -> lockmod.SymbolSet:
    return lockmod.SymbolSet.from_json(json.loads((OP_NORTH / "scenario.lock.json").read_text())["symbols"])


def test_canonical_json_is_compact_and_sorted():
    raw = lockmod.canonical_json("op_north", 3, op_north_symbols())
    assert raw.startswith(b'{"scenario":"op_north","schema":3,"symbols":{"airbases":[{"ident":"MAYKOP_KHANSKAYA"}]')
    assert b" " not in raw.replace(b"Smoke the objective", b"")
    assert lockmod.schema_sha256("op_north", 3, op_north_symbols()) == OP_NORTH_SCHEMA_SHA256


def test_parse_lock_rejects_tampering():
    text = (OP_NORTH / "scenario.lock.json").read_text()
    assert lockmod.parse_lock(text)["scenario"] == "op_north"
    with pytest.raises(LockError, match="mismatch"):
        lockmod.parse_lock(text.replace('"SAM_RING"\n', '"SAM_RINGS"\n'))
    with pytest.raises(LockError, match="format"):
        lockmod.parse_lock(text.replace('"format": 1', '"format": 2'))
    with pytest.raises(LockError, match="unknown keys"):
        lockmod.parse_lock(text.replace('"format": 1', '"format": 1, "extra": 0'))
    with pytest.raises(LockError, match="invalid"):
        lockmod.parse_lock("{not json")


def test_lock_diff_names_what_changed():
    old = json.loads((OP_NORTH / "scenario.lock.json").read_text())
    new = json.loads(json.dumps(old))
    assert lockmod.diff_locks(old, new).is_empty
    new["symbols"]["flags"].append({"ident": "EXTRA"})
    new["schema_sha256"] = lockmod.schema_sha256("op_north", 3, lockmod.SymbolSet.from_json(new["symbols"]))
    d = lockmod.diff_locks(old, new)
    assert d.hash_changed and d.added == ("flag:EXTRA",) and "new symbols: flag:EXTRA" in d.describe()
    moved = json.loads(json.dumps(old))
    moved["geometry"]["zones"]["OBJECTIVE"]["circle"]["radius_m"] = 900.0
    d = lockmod.diff_locks(old, moved)
    assert not d.hash_changed and d.geometry_changed and d.describe() == "zone geometry changed"


def test_symbol_set_checks_kinds_and_duplicates():
    s = op_north_symbols()
    s.check()
    bad = lockmod.SymbolSet.from_json({"groups": [{"ident": "A", "kind": "tank", "roles": []}]})
    with pytest.raises(LockError, match="unknown kind"):
        bad.check()
    dup = lockmod.SymbolSet.from_json({"zones": [{"ident": "Z"}, {"ident": "Z"}]})
    with pytest.raises(LockError, match="duplicate"):
        dup.check()
    with pytest.raises(InvalidName):
        lockmod.SymbolSet.from_json({"flags": [{"ident": "lower"}]}).check()


def test_ulid_round_trip_and_content_ids():
    when = datetime(2026, 11, 1, 6, 30, tzinfo=timezone.utc)
    a = ulid.content_id(when, b"manifest")
    assert ulid.is_ulid(a) and len(a) == 26
    assert a == ulid.content_id(when, b"manifest")
    assert a != ulid.content_id(when, b"other manifest")
    assert ulid.timestamp_ms(a) == int(when.timestamp() * 1000)
    assert ulid.encode(ulid.decode(a)) == a
    assert ulid.is_ulid(ulid.new())
    assert not ulid.is_ulid("01K8Y2Q0PH000000000000000I")  # I is not Crockford base32
    assert not ulid.is_ulid("81K8Y2Q0PH0000000000000000")  # overflows 128 bits
    with pytest.raises(ValueError):
        ulid.content_id(datetime(2026, 1, 1), b"x")


def test_json_value_turns_lua_arrays_into_json_arrays():
    assert _pydcs.to_json_value({1: "a", 2: "b"}) == ["a", "b"]
    # Holes stay holes, so the engine rebuilds the same Lua indexes.
    assert _pydcs.to_json_value({1: "a", 3: "c"}) == ["a", None, "c"]
    assert _pydcs.to_json_value({"x": -0.0, "n": [1, 2]}) == {"x": 0.0, "n": [1, 2]}
    with pytest.raises(_pydcs.LuaShapeError):
        _pydcs.to_json_value({1: "a", "b": 2})
    with pytest.raises(_pydcs.LuaShapeError):
        _pydcs.to_json_value({0: "zero"})
    with pytest.raises(_pydcs.LuaShapeError):
        _pydcs.to_json_value({"x": float("nan")})


def test_atomic_writes_leave_no_temp_files(tmp_path):
    from sortie_gen.fsutil import atomic_write_bytes, atomic_write_text, write_if_changed

    target = tmp_path / "sub" / "out.bin"
    atomic_write_bytes(target, b"one")
    atomic_write_bytes(target, b"two")
    assert target.read_bytes() == b"two"
    assert write_if_changed(target, b"two") is False
    assert write_if_changed(target, b"three") is True
    atomic_write_text(tmp_path / "t.json", "{}\n")
    assert (tmp_path / "t.json").read_bytes() == b"{}\n"
    assert sorted(p.name for p in tmp_path.rglob("*") if p.is_file()) == ["out.bin", "t.json"]
