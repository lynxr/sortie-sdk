"""Errors with messages meant for the scenario author (the CLI prints them without a traceback)."""

from __future__ import annotations


class GeneratorError(Exception):
    """Base class: something in the scenario or the environment needs fixing."""

    exit_code = 1


class InvalidName(GeneratorError, ValueError):
    """A scenario name, ident, role or DCS name breaks the naming rules."""


class NameConflict(GeneratorError, ValueError):
    """Two objects would share a name or an ident (pydcs does not check; DCS would destroy one)."""


class ScenarioError(GeneratorError, ValueError):
    """A scenario declaration is incomplete or contradictory."""


class LockError(GeneratorError, ValueError):
    """scenario.lock.json is malformed, tampered with, or stale where it must be current."""


class EngineError(GeneratorError):
    """The engine answered with an error, or could not be reached."""


class EngineUnavailable(EngineError):
    """No engine is listening at the configured address."""


class EngineNotReady(EngineError):
    """The engine answered FAILED_PRECONDITION: the request is fine, but DCS is not in a state
    to serve it (no mission running, paused, a link down); ``details`` says which."""

    def __init__(self, method: str, details: str) -> None:
        self.method = method
        self.details = details
        super().__init__(f"{method} failed: FAILED_PRECONDITION: {details}")


class CommandFailed(EngineError):
    """A command reached the engine and failed: ``code`` is the ``ErrorCode`` name without its
    prefix (``"REJECTED"``, ``"INVALID"``, ``"GONE"``, ...), ``capability`` names the switched-off
    or missing capability (``"call:mission:Airbase.setCoalition"``) when there is one."""

    def __init__(self, op: str, code: str, message: str, capability: str = "") -> None:
        self.op = op
        self.code = code
        self.capability = capability
        extra = f" (capability {capability})" if capability else ""
        super().__init__(f"{op} failed: {code}: {message}{extra}")


class ResultNotKept(EngineError):
    """A keyed universal call was answered from the engine's key store, which kept no values
    for it: the call ran under this key before, it was not run again, and its result was above
    64 KiB (the engine stores such a result without its values)."""

    def __init__(self, op: str) -> None:
        self.op = op
        super().__init__(
            f"{op}: the call ran under this key before and was not run again, and the engine kept no "
            "values for it (a result above 64 KiB is stored without them)"
        )


class UnknownTypes(ScenarioError):
    """DCS on the server does not know some unit or static types the scenario uses."""


class NotYet(EngineError):
    """A feature that is deliberately out of scope for now; the message names its milestone."""

    exit_code = 2

    def __init__(self, feature: str, milestone: str | None = None, detail: str | None = None) -> None:
        self.feature = feature
        self.milestone = milestone
        text = detail or (f"{feature} lands in {milestone}" if milestone else feature)
        super().__init__(text)
