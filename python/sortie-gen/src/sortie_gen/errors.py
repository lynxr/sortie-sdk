"""Errors with messages meant for the scenario author (the CLI prints them without a traceback)."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


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


@dataclass(frozen=True)
class Limit:
    """Which of the engine's limits refused a command, or what it waited for until its deadline
    (``CommandError.limit``, engine 0.4.0). ``kind`` is the ``LimitKind`` name without its
    prefix (``"CALL_RATE"``, ``"DCS_TIME"``, ``"BATCH_SIZE"``, ``"TASKING_DELAY"``, ...),
    ``budget`` its stable name (``"call_rate.read"``, ``"dcs_time.read"``, ...), ``writer`` whose
    budget it is (empty for a limit of the session or of DCS); the numbers are whole, in the
    budget's unit; ``retry_after`` is in seconds, ``None`` where no refill can tell."""

    kind: str
    budget: str
    writer: str = ""
    capacity: int = 0
    per_second: int = 0
    needed: int = 0
    available: int = 0
    retry_after: float | None = None

    @classmethod
    def from_proto(cls, limit: Any) -> Limit:
        """The ``common_pb2.Limit`` of a ``CommandError``."""
        from ._proto.sortie.v1 import common_pb2

        try:
            kind = common_pb2.LimitKind.Name(limit.kind).removeprefix("LIMIT_KIND_")
        except ValueError:
            kind = "UNSPECIFIED"
        return cls(
            kind=kind,
            budget=limit.budget,
            writer=limit.writer,
            capacity=limit.capacity,
            per_second=limit.per_second,
            needed=limit.needed,
            available=limit.available,
            retry_after=limit.retry_after_ms / 1000.0 if limit.retry_after_ms else None,
        )


class CommandFailed(EngineError):
    """A command reached the engine and failed: ``code`` is the ``ErrorCode`` name without its
    prefix (``"REJECTED"``, ``"INVALID"``, ``"GONE"``, ...), ``capability`` names the switched-off
    or missing capability (``"call:mission:Airbase.setCoalition"``) when there is one.

    ``limit`` (engine 0.4.0) says which limit refused the command, or what it waited for until
    its deadline (a ``TIMEOUT`` with a limit never ran in DCS); ``retry_after`` is when the same
    command, sent again unchanged, passes the rate budgets (seconds; ``None`` without an
    estimate). A forbid (a path not in the catalog or switched off) has ``capability`` and no
    ``limit``: ``forbidden`` is true then, and no retry helps."""

    def __init__(
        self, op: str, code: str, message: str, capability: str = "", limit: Limit | None = None
    ) -> None:
        self.op = op
        self.code = code
        self.message = message
        self.capability = capability
        self.limit = limit
        extra = f" (capability {capability})" if capability else ""
        if limit is not None:
            extra += f" (limit {limit.budget})"
        super().__init__(f"{op} failed: {code}: {message}{extra}")

    @property
    def retry_after(self) -> float | None:
        """Seconds after the refusal at which the same command passes its rate budgets."""
        return self.limit.retry_after if self.limit is not None else None

    @property
    def forbidden(self) -> bool:
        """A forbid: ``REJECTED`` with a capability and no limit; no retry helps."""
        return self.code == "REJECTED" and bool(self.capability) and self.limit is None


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
