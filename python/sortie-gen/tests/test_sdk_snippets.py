"""The SDK's checked Python examples (``sdk/snippets/python``) and its mission template.

* every snippet file compiles (``py_compile``), the skills' ones included;
* every scenario a ``docs_*.py`` file defines builds offline without warnings;
* the template's scenario builds, and the committed locks of the template and of the docs'
  scenario (the one the Rust snippets are generated from) are what the generator writes.

``cargo xtask docs-check`` compares these files with the code blocks of the docs; ``cargo xtask
fixtures`` rewrites the locks. Skipped where there is no ``sdk/`` folder (an exported SDK).
"""

from __future__ import annotations

import py_compile
import runpy
import warnings
from pathlib import Path

import pytest

from conftest import REPO
from sortie_gen import Scenario, SortieWarning

SDK = REPO / "sdk"
SNIPPETS = SDK / "snippets" / "python"
TEMPLATE = SDK / "template"

pytestmark = pytest.mark.skipif(not SNIPPETS.is_dir(), reason="no sdk/snippets here (an exported SDK)")


def snippet_files(prefix: str = "") -> list[Path]:
    return sorted(p for p in SNIPPETS.glob(f"{prefix}*.py") if p.is_file())


def scenarios_of(path: Path) -> list[Scenario]:
    """Runs a scenario file without its ``__main__`` block; the Scenario objects it defines."""
    module = runpy.run_path(str(path), run_name="__sortie_snippet__")
    found = [v for v in module.values() if isinstance(v, Scenario)]
    return list({id(s): s for s in found}.values())


def build_cleanly(sc: Scenario):
    with warnings.catch_warnings(record=True) as records:
        warnings.simplefilter("always", SortieWarning)
        b = sc.build()
    sortie = [str(r.message) for r in records if issubclass(r.category, SortieWarning)]
    assert sortie == [], f"{sc.name}: {sortie}"
    return b


def test_there_are_snippets():
    assert snippet_files("docs_"), f"no docs_*.py in {SNIPPETS}"


@pytest.mark.parametrize("path", snippet_files(), ids=lambda p: p.name)
def test_every_python_snippet_compiles(path: Path, tmp_path: Path):
    py_compile.compile(str(path), cfile=str(tmp_path / "out.pyc"), doraise=True)


@pytest.mark.parametrize("path", snippet_files("docs_"), ids=lambda p: p.name)
def test_docs_snippet_scenarios_build(path: Path):
    scenarios = scenarios_of(path)
    assert scenarios, f"{path.name} defines no Scenario"
    for sc in scenarios:
        b = build_cleanly(sc)
        assert b.miz and b.manifest_pb and b.build_id


@pytest.mark.parametrize(
    ("scenario", "lock"),
    [
        (TEMPLATE / "scenario.py", TEMPLATE / "scenario.lock.json"),
        (SNIPPETS / "docs_scenario.py", SDK / "snippets" / "scenario" / "scenario.lock.json"),
    ],
    ids=["template", "docs"],
)
def test_committed_locks_are_current(scenario: Path, lock: Path):
    [sc] = scenarios_of(scenario)
    build_cleanly(sc)
    assert lock.read_text(encoding="utf-8") == sc.lock_text(), (
        f"{lock.relative_to(REPO)} is stale: run `cargo xtask fixtures`"
    )


def test_the_template_and_its_docs_copy_are_the_same_scenario():
    [template] = scenarios_of(TEMPLATE / "scenario.py")
    [copy] = scenarios_of(SNIPPETS / "docs_template.py")
    assert template.lock_text() == copy.lock_text()
    assert template.build().build_id == copy.build().build_id


def test_the_docs_scenario_has_every_symbol_of_the_template():
    """The Rust snippets compile the template's mission against the docs' scenario."""
    [template] = scenarios_of(TEMPLATE / "scenario.py")
    [docs] = scenarios_of(SNIPPETS / "docs_scenario.py")
    assert template.name == docs.name
    t, d = template.symbols(), docs.symbols()
    assert set(t.zones) <= set(d.zones)
    assert {(g.ident, g.kind) for g in t.groups} <= {(g.ident, g.kind) for g in d.groups}
