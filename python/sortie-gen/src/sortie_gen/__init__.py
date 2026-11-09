"""sortie-gen: the Sortie mission generator.

Write a scenario with :class:`Scenario` (a thin wrapper over ``dcs.Mission``), then

* ``python scenario.py lock``  -> ``scenario.lock.json`` (symbols for the Rust logic);
* ``python scenario.py build`` -> ``dist/<build_id>.miz`` + ``.manifest.pb`` + ``.manifest.json``;
* ``python scenario.py validate | register | launch | watch | results`` -> talk to the engine
  (sortied); ``register`` and ``launch`` check the unit types with the mission running in DCS
  first.

The generator guide (Russian) is part of the Sortie docs.
"""

from __future__ import annotations

__version__ = "0.1.0"

from . import _pydcs

_pydcs.configure()

from .errors import (  # noqa: E402
    EngineError,
    EngineNotReady,
    EngineUnavailable,
    GeneratorError,
    InvalidName,
    LockError,
    NameConflict,
    NotYet,
    ScenarioError,
    UnknownTypes,
)
from .scenario import Build, Coalition, Scenario, SortieWarning, StartType, TypeUse  # noqa: E402

__all__ = [
    "Build",
    "Coalition",
    "EngineError",
    "EngineNotReady",
    "EngineUnavailable",
    "GeneratorError",
    "InvalidName",
    "LockError",
    "NameConflict",
    "NotYet",
    "Scenario",
    "ScenarioError",
    "SortieWarning",
    "StartType",
    "TypeUse",
    "UnknownTypes",
    "__version__",
]
