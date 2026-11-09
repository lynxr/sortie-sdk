from __future__ import annotations

import io
from pathlib import Path

import pytest

from sortie_gen.cli import Ui, load_scenario

PACKAGE = Path(__file__).resolve().parents[1]
REPO = PACKAGE.parents[1]
OP_NORTH = REPO / "examples" / "op_north"
OP_NORTH_SCENARIO = OP_NORTH / "scenario.py"
FIXTURES = OP_NORTH / "logic" / "fixtures"

# The schema hash of op_north that the Rust crates hard-code.
OP_NORTH_SCHEMA_SHA256 = "066934cb939700310641d6371b1f6140cfc86c2c3e04757d5a10f8a4664f270b"


class CapturedUi(Ui):
    def __init__(self) -> None:
        super().__init__(io.StringIO(), io.StringIO())

    @property
    def stdout(self) -> str:
        return self.out.getvalue()  # type: ignore[attr-defined]

    @property
    def stderr(self) -> str:
        return self.err.getvalue()  # type: ignore[attr-defined]


@pytest.fixture
def ui() -> CapturedUi:
    return CapturedUi()


@pytest.fixture(scope="session")
def op_north():
    """The op_north Scenario, loaded from the example file without running its CLI."""
    return load_scenario(OP_NORTH_SCENARIO)


@pytest.fixture(scope="session")
def op_north_build(op_north):
    import warnings

    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return op_north.build()
