"""The table ``sortie.static_type`` (catalog/static_types.toml) is current and well formed."""

from __future__ import annotations

import tomllib
from pathlib import Path

from sortie_gen.static_types import HELIPORTS, UNIT_CATEGORIES, render, render_notice, rows

TABLE = Path(__file__).resolve().parents[3] / "catalog" / "static_types.toml"


def test_the_committed_table_is_current() -> None:
    assert TABLE.read_bytes().decode("utf-8") == render(), (
        "catalog/static_types.toml is stale: run `uv run python scripts/gen_static_types.py`"
    )


def test_rows_pair_every_type_with_its_category_and_shape() -> None:
    table = rows()
    assert table["Cafe"] == ("Fortifications", "stolovaya")
    assert table["ammo_cargo"][0] == "Cargos"
    assert table["Mi-8MT"] == ("Helicopters", "")
    for type_name, shape in HELIPORTS.items():
        assert table[type_name] == ("Heliports", shape)
    for type_name, (category, shape) in table.items():
        assert category, type_name
        # Unit types take the unit's model; every other static needs its shape.
        assert (shape == "") == (category in UNIT_CATEGORIES), type_name


def test_the_file_parses_as_the_engine_reads_it() -> None:
    doc = tomllib.loads(TABLE.read_text(encoding="utf-8"))
    assert doc["version"] == 1
    types = doc["types"]
    assert len(types) == len(rows())
    assert types["Invisible FARP"] == {"category": "Heliports", "shape": "invisiblefarp"}
    assert "shape" not in types["Mi-8MT"]

def test_the_committed_notice_is_current() -> None:
    notice = TABLE.parent / "THIRD-PARTY-pydcs.txt"
    assert notice.read_bytes().decode("utf-8") == render_notice(), (
        "catalog/THIRD-PARTY-pydcs.txt is stale: run `uv run python scripts/gen_static_types.py`"
    )
    assert "GNU LESSER GENERAL PUBLIC LICENSE" in notice.read_text(encoding="utf-8")


def test_the_table_holds_facts_only_and_names_its_source() -> None:
    text = TABLE.read_text(encoding="utf-8")
    head = text.split("\nversion = 1\n", 1)[0]
    assert "pydcs 1.0.0" in head and "LGPL-3.0" in head
    assert "scripts/gen_static_types.py" in head and "--check" in head
    body = text.split("[types]\n", 1)[1]
    for line in body.splitlines():
        # `"<type>" = { category = "..", shape = ".." }` and nothing else (no comments).
        assert not line.lstrip().startswith("#"), line
        key, value = line.split(" = ", 1)
        assert value.startswith("{ category = ") and value.endswith(" }"), line
