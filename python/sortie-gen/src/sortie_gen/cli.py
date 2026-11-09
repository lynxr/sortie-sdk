"""Command lines: ``python scenario.py <command>`` (``Scenario.cli``) and ``sortie-gen``.

Scenario commands::

    lock      write scenario.lock.json (symbols for the Rust logic); --check for CI
    build     write dist/<build_id>.miz + .manifest.pb + .manifest.json (offline)
    validate  check every unit and static type with the mission running in DCS
    register  BeginBuild + build + type check + Missions.Register (the engine writes the .miz)
    launch    load a registered build in DCS (no --build-id: register a fresh build first)
    restart   reload the mission DCS runs (its session ends as restarted, a new one starts)
    watch     print the engine's live events
    results   print the results of the last (or a given) session

``sortie-gen`` adds ``lobby``, ``worker`` and ``adopt``, and runs the scenario commands on a
file: ``sortie-gen build examples/op_north/scenario.py``.

Errors meant for the user are printed as ``error: ...`` without a traceback; the exit code is 2
for features that land in a later milestone (like the Rust CLI), 1 for other errors.
"""

from __future__ import annotations

import argparse
import json
import runpy
import sys
import warnings
from collections.abc import Sequence
from pathlib import Path
from typing import Any, TextIO

from google.protobuf import json_format

from . import __version__
from . import lock as lockmod
from ._proto.sortie.v1 import common_pb2 as cpb
from .errors import EngineError, EngineNotReady, GeneratorError, LockError, NotYet, UnknownTypes
from .fsutil import atomic_write_bytes, write_if_changed
from .scenario import Build, Scenario, SortieWarning, TypeUse

_EVENT_KINDS = {
    "birth": 1, "dead": 2, "unit_lost": 3, "kill": 4, "hit": 5, "shot": 6, "takeoff": 7, "land": 8,
    "base_captured": 9, "mark": 10, "player_entered_unit": 11, "unit_gone": 12, "menu_pressed": 13,
    "player_connected": 14, "player_disconnected": 15, "player_changed_slot": 16, "chat_message": 17,
    "simulation_paused": 18, "simulation_resumed": 19, "gap": 20, "unknown": 21,
}  # fmt: skip


class Ui:
    """Where the CLI prints (replaceable in tests)."""

    def __init__(self, out: TextIO | None = None, err: TextIO | None = None) -> None:
        self.out = out or sys.stdout
        self.err = err or sys.stderr

    def say(self, text: str = "") -> None:
        print(text, file=self.out, flush=True)

    def warn(self, text: str) -> None:
        print(f"warning: {text}", file=self.err, flush=True)

    def error(self, text: str) -> None:
        print(f"error: {text}", file=self.err, flush=True)


def _rel(p: Path) -> str:
    try:
        return str(p.resolve().relative_to(Path.cwd().resolve()))
    except ValueError:
        return str(p)


def _size(n: int) -> str:
    return f"{n / 1024:.1f} KiB" if n < 1024 * 1024 else f"{n / 1024 / 1024:.1f} MiB"


# ----- argument parsing --------------------------------------------------------------------


def _engine_args(p: argparse.ArgumentParser) -> None:
    p.add_argument("--engine", help="engine API address (default: $SORTIE_ENGINE or 127.0.0.1:50777)")
    p.add_argument("--token", help="API token (default: $SORTIE_TOKEN; not needed with `sortied --dev`)")
    p.add_argument("--timeout", type=float, default=15.0, help="seconds per call (default 15)")


def _validate_arg(p: argparse.ArgumentParser) -> None:
    p.add_argument(
        "--no-validate",
        action="store_true",
        help="skip the check of unit and static types with the mission running in DCS (Generator.ValidateTypes)",
    )


def _build_args(p: argparse.ArgumentParser, *, out_help: str = "output folder (default: dist/ next to the scenario)") -> None:
    p.add_argument("--out", type=Path, help=out_help)
    p.add_argument("--lock", type=Path, help="lock file (default: scenario.lock.json next to the scenario)")
    p.add_argument("--locked", action="store_true", help="fail instead of updating a stale scenario.lock.json (CI)")


def scenario_parser(prog: str) -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(prog=prog, description="Sortie scenario commands")
    ap.add_argument("--version", action="version", version=f"sortie-gen {__version__}")
    sub = ap.add_subparsers(dest="command", required=True, metavar="COMMAND")

    p = sub.add_parser("lock", help="write scenario.lock.json")
    p.add_argument("--out", type=Path, help="lock file (default: scenario.lock.json next to the scenario)")
    p.add_argument("--check", action="store_true", help="only check that the lock file is current (exit 1 if not)")

    p = sub.add_parser("build", help="build the .miz and the manifest offline")
    _build_args(p)
    p.add_argument("--build-id", help="use this build id instead of the content-derived one")
    p.add_argument("--stem", help="file name stem (default: the build id)")
    p.add_argument("--no-miz", action="store_true", help="write only the manifest files")

    p = sub.add_parser("validate", help="check the unit and static types with the mission running in DCS")
    _engine_args(p)

    p = sub.add_parser("register", help="build with an engine build id and register it with the engine")
    _build_args(p, out_help="also write the build here (the engine keeps its own copy under Missions/Sortie)")
    _validate_arg(p)
    _engine_args(p)

    p = sub.add_parser("launch", help="launch a registered build (default: register a fresh one first)")
    p.add_argument("--build-id", help="a build registered earlier")
    _build_args(p, out_help="also write the build here (the engine keeps its own copy under Missions/Sortie)")
    _validate_arg(p)
    _engine_args(p)

    p = sub.add_parser("restart", help="reload the mission DCS runs (its session ends as restarted)")
    _engine_args(p)

    p = sub.add_parser("watch", help="print live events from the engine (Ctrl-C to stop)")
    p.add_argument("--session", default="", help="session id (default: the current one)")
    p.add_argument("--kinds", default="", help=f"comma-separated event kinds: {', '.join(_EVENT_KINDS)}")
    p.add_argument("--from-seq", type=int, default=0, help="replay from this event number first")
    _engine_args(p)

    p = sub.add_parser("results", help="print mission results")
    p.add_argument("--session", default="", help="session id (default: the last ended one)")
    p.add_argument("--json", action="store_true", help="print the MissionResults message as JSON")
    _engine_args(p)
    return ap


def _client(args: argparse.Namespace) -> Any:
    from .engine import EngineClient

    return EngineClient(args.engine, args.token, args.timeout)


# ----- commands ------------------------------------------------------------------------------


def _build_with_warnings(sc: Scenario, ui: Ui, build_id: str | None) -> Build:
    with warnings.catch_warnings(record=True) as records:
        warnings.simplefilter("always", SortieWarning)
        b = sc.build(build_id)
    for r in records:
        if issubclass(r.category, SortieWarning):
            ui.warn(str(r.message))
        else:
            warnings.warn_explicit(r.message, r.category, r.filename, r.lineno)
    return b


def _sync_lock(sc: Scenario, ui: Ui, path: Path, *, locked: bool, check_only: bool = False) -> bool:
    """Writes the lock if it changed; returns True if it was current. Raises with --locked."""
    new = sc.lock()
    text = lockmod.lock_text(new)
    existed = path.exists()
    raw = path.read_text(encoding="utf-8") if existed else None
    if raw == text:
        return True
    old: dict[str, Any] | None = None
    invalid: str | None = None
    if raw is not None:
        try:
            old = lockmod.parse_lock(raw)
        except LockError as e:
            invalid = str(e)
            try:  # still describe the change against what the file says
                loose = json.loads(raw)
                old = loose if isinstance(loose, dict) and isinstance(loose.get("symbols"), dict) else None
            except json.JSONDecodeError:
                old = None
    try:
        diff = lockmod.diff_locks(old, new)
    except (LockError, KeyError, TypeError, ValueError):
        diff = lockmod.diff_locks(None, new)
    what_changed = diff.describe()
    if invalid:
        what_changed = f"the file is invalid: {invalid}; {what_changed}"
    if locked or check_only:
        raise LockError(
            f"{_rel(path)} is stale ({what_changed}); run `python {sc.source.name if sc.source else 'scenario.py'} lock`"
        )
    write_if_changed(path, text.encode("utf-8"))
    ui.say(f"{'updated' if existed else 'wrote'} {_rel(path)}: {what_changed} (schema {sc.schema}, hash {new['schema_sha256'][:12]})")
    if old is not None and diff.hash_changed:
        ui.warn("the symbol set changed: rebuild the mission logic (cargo build) so it sees the new symbols")
    return False


def cmd_lock(sc: Scenario, args: argparse.Namespace, ui: Ui) -> int:
    path = args.out or sc.default_lock_path()
    if _sync_lock(sc, ui, path, locked=False, check_only=args.check):
        ui.say(f"{_rel(path)} is up to date (hash {sc.schema_sha256()[:12]})")
    return 0


def _write_build(sc: Scenario, b: Build, args: argparse.Namespace, ui: Ui, *, stem: str | None, miz: bool) -> dict[str, Path]:
    out = args.out or sc.default_out_dir()
    files = b.write(out, stem=stem, miz=miz)
    ui.say(f"built {sc.name} {b.build_id} (schema {sc.schema}, hash {b.manifest.schema_sha256[:12]})")
    for key in ("miz", "pb", "json"):
        if key in files:
            p = files[key]
            extra = f"  digest {b.digest[:16]}" if key == "pb" else ""
            ui.say(f"  {_rel(p):<60} {_size(p.stat().st_size)}{extra}")
    return files


def cmd_build(sc: Scenario, args: argparse.Namespace, ui: Ui) -> int:
    _sync_lock(sc, ui, args.lock or sc.default_lock_path(), locked=args.locked)
    b = _build_with_warnings(sc, ui, args.build_id)
    _write_build(sc, b, args, ui, stem=args.stem, miz=not args.no_miz)
    return 0


# ----- type check (Generator.ValidateTypes) ----------------------------------------------------


def _category(value: int) -> str:
    if value == cpb.CATEGORY_UNSPECIFIED:
        return "-"
    try:
        return cpb.Category.Name(value).removeprefix("CATEGORY_").lower().replace("_", " ")
    except ValueError:
        return str(value)


def _places(uses: list[TypeUse], most: int = 3) -> str:
    where = [u.where for u in uses]
    if len(where) <= most:
        return ", ".join(where)
    return ", ".join(where[:most]) + f" and {len(where) - most} more"


def _check_types(b: Build, client: Any) -> tuple[dict[str, Any], dict[str, list[TypeUse]]]:
    """Asks the engine about every type of the build: ``(type -> TypeCheck, type -> uses)``."""
    uses: dict[str, list[TypeUse]] = {}
    for u in b.type_uses:
        uses.setdefault(u.type_name, []).append(u)
    r = client.validate_types(uses.keys())
    return {c.type_name: c for c in r.results}, uses


def _judge_types(checks: dict[str, Any], uses: dict[str, list[TypeUse]], ui: Ui) -> int:
    """Warns about doubtful types and raises :class:`UnknownTypes` for unknown ones; returns
    how many types DCS knows."""
    unknown: list[str] = []
    for name, used in sorted(uses.items()):
        c = checks.get(name)
        if c is None:
            ui.warn(f"the engine did not answer for type {name!r} ({_places(used)}); it was not checked")
        elif not c.known:
            unknown.append(f"  {name!r} ({_places(used)})")
        elif c.category == cpb.CATEGORY_STATIC:
            as_unit = [u for u in used if not u.static]
            if as_unit:
                ui.warn(
                    f"{name!r} is a unit of {_places(as_unit)}, but DCS knows it only as a static "
                    "object type (StaticObject.getDescByName, not Unit.getDescByName)"
                )
    if unknown:
        raise UnknownTypes(
            f"DCS on the server does not know {len(unknown)} type(s) this scenario uses:\n"
            + "\n".join(unknown)
            + "\nIs the type name misspelled, or is the module or mod that provides it not installed "
            "on the DCS server? Fix the scenario or install it there (--no-validate skips this check)."
        )
    return sum(1 for name in uses if name in checks and checks[name].known)


def _validate_before_register(b: Build, ui: Ui, client: Any) -> None:
    """Stops on types DCS does not know; when the check cannot run, warns and goes on."""
    if not b.type_uses:
        return
    try:
        checks, uses = _check_types(b, client)
    except EngineNotReady as e:
        ui.warn(f"unit and static types not checked: {e.details} (registering anyway)")
        return
    except NotYet as e:
        ui.warn(f"unit and static types not checked: this engine cannot check them yet ({e}); registering anyway")
        return
    except EngineError as e:
        raise EngineError(f"{e} (--no-validate skips the type check)") from None
    known = _judge_types(checks, uses, ui)
    ui.say(f"checked {len(uses)} DCS type(s) with the mission running in DCS: {known} known")


def cmd_validate(sc: Scenario, args: argparse.Namespace, ui: Ui) -> int:
    b = _build_with_warnings(sc, ui, None)
    if not b.type_uses:
        ui.say(f"{sc.name} uses no unit or static types")
        return 0
    with _client(args) as client:
        checks, uses = _check_types(b, client)
    width = max(len(repr(n)) for n in uses) + 2
    ui.say(f"{'type':<{width}} {'known':<6} {'category':<12} used by")
    for name, used in sorted(uses.items()):
        c = checks.get(name)
        known = "?" if c is None else ("yes" if c.known else "NO")
        category = "-" if c is None else _category(c.category)
        ui.say(f"{name!r:<{width}} {known:<6} {category:<12} {_places(used)}")
    known_n = _judge_types(checks, uses, ui)
    ui.say(f"{known_n} of {len(uses)} type(s) are known to the mission running in DCS")
    return 0


def _register(sc: Scenario, args: argparse.Namespace, ui: Ui, client: Any) -> Build:
    _sync_lock(sc, ui, args.lock or sc.default_lock_path(), locked=args.locked)
    build_id = client.begin_build(sc.name, sc.schema, sc.schema_sha256(), sc.generator_string())
    b = _build_with_warnings(sc, ui, build_id)
    if not args.no_validate:
        _validate_before_register(b, ui, client)
    if args.out is not None:
        _write_build(sc, b, args, ui, stem=None, miz=True)
    else:
        ui.say(f"built {sc.name} {b.build_id} (schema {sc.schema}, hash {b.manifest.schema_sha256[:12]}, {_size(len(b.miz))})")
    reg = client.register(b.build_id, b.manifest_pb, b.miz, f"{sc.name}-{b.build_id}.miz")
    if reg.digest and reg.digest != b.digest:
        raise GeneratorError(f"the engine computed digest {reg.digest}, the bootstrap carries {b.digest}")
    ui.say(f"registered {b.build_id} with the engine at {client.address}: {reg.path}")
    return b


def cmd_register(sc: Scenario, args: argparse.Namespace, ui: Ui) -> int:
    with _client(args) as client:
        _register(sc, args, ui, client)
    return 0


def cmd_launch(sc: Scenario, args: argparse.Namespace, ui: Ui) -> int:
    with _client(args) as client:
        build_id = args.build_id or _register(sc, args, ui, client).build_id
        r = client.launch(build_id)
        ui.say(f"launched {r.build_id or build_id}: DCS loads {r.path}")
        ui.say("follow it with `watch`; the session's results appear under `results` when it ends")
    return 0


def cmd_restart(sc: Scenario | None, args: argparse.Namespace, ui: Ui) -> int:
    with _client(args) as client:
        r = client.restart()
    if r.ended_session:
        ui.say(f"restarting: session {r.ended_session} ends (restart); DCS reloads its mission")
        ui.say(f"its results: `results --session {r.ended_session}`; the new session starts when the mission is up")
    else:
        ui.say("restarting: no session was running; DCS reloads the mission it runs")
    ui.say("follow it with `watch`")
    return 0


def describe_event(ev: Any) -> str:
    kind = ev.WhichOneof("kind") or "unknown"
    body = getattr(ev, kind)
    parts = [f"#{ev.seq:<5} t={ev.model_time:9.1f}  {kind.upper():<20}"]
    for field_name in ("initiator", "target", "unit", "last_known"):
        if body.DESCRIPTOR.fields_by_name.get(field_name) is not None and body.HasField(field_name):
            e = getattr(body, field_name)
            who = e.name or f"eid {e.eid}"
            if e.player:
                who += f" [{e.player}]"
            parts.append(f"{field_name}={who}")
    for field_name in ("name", "text", "ident", "reason", "dcs_name"):
        f = body.DESCRIPTOR.fields_by_name.get(field_name)
        if f is not None and f.type == f.TYPE_STRING and getattr(body, field_name):
            parts.append(f"{field_name}={getattr(body, field_name)!r}")
    return "  ".join(parts)


def cmd_watch(sc: Scenario | None, args: argparse.Namespace, ui: Ui) -> int:
    kinds = []
    for k in filter(None, (s.strip().lower() for s in args.kinds.split(","))):
        if k not in _EVENT_KINDS:
            raise GeneratorError(f"unknown event kind {k!r} (one of {', '.join(_EVENT_KINDS)})")
        kinds.append(_EVENT_KINDS[k])
    with _client(args) as client:
        h = client.health()
        status = {1: "ok", 2: "degraded", 3: "down"}.get(h.status, "unknown")
        links = ", ".join(f"{link.env}: {'up' if link.connected else 'down'}" for link in h.links) or "no bridge links"
        ui.say(
            f"engine {client.address}: {status}; {links}; session {h.session or '-'}"
            + (f" ({h.scenario}, t={h.model_time:.0f}{', paused' if h.paused else ''})" if h.session else "")
        )
        for note in h.notes:
            ui.say(f"  note: {note}")
        try:
            for ev in client.events(args.session, kinds, args.from_seq):
                ui.say(describe_event(ev))
        except KeyboardInterrupt:
            pass
    return 0


_OUTCOMES = {0: "-", 1: "blue win", 2: "red win", 3: "draw", 4: "aborted", 5: "time limit"}
_END_REASONS = {0: "-", 1: "logic", 2: "time limit", 3: "restart", 4: "mission unloaded", 5: "engine shutdown"}
_FATES = {0: "?", 1: "alive", 2: "dead", 3: "gone", 4: "never seen"}


def cmd_results(sc: Scenario | None, args: argparse.Namespace, ui: Ui) -> int:
    with _client(args) as client:
        r = client.results(args.session)
    if args.json:
        ui.say(json_format.MessageToJson(r, preserving_proto_field_name=True, indent=2, sort_keys=True))
        return 0
    ui.say(f"session {r.session}: {r.scenario} {r.build_id}")
    ui.say(f"  outcome {_OUTCOMES.get(r.outcome, r.outcome)} ({_END_REASONS.get(r.end_reason, r.end_reason)}), t={r.model_time:.0f} s")
    if r.text:
        ui.say(f"  {r.text}")
    by_group: dict[str, list[Any]] = {}
    for f in r.fates:
        by_group.setdefault(f.group_ident or f.group_name or "-", []).append(f)
    for group, fates in sorted(by_group.items()):
        counts: dict[str, int] = {}
        for f in fates:
            counts[_FATES.get(f.fate, "?")] = counts.get(_FATES.get(f.fate, "?"), 0) + 1
        ui.say(f"  {group:<24} " + ", ".join(f"{n} {k}" for k, n in sorted(counts.items())))
    for k in r.kills:
        ui.say(f"  kill t={k.model_time:.0f}: {k.killer or '?'} -> {k.victim} ({k.weapon or '?'})")
    for name, value in sorted(r.flags.items()):
        ui.say(f"  flag {name} = {value}")
    for key, value in sorted(r.report_json.items()):
        ui.say(f"  report {key} = {value}")
    return 0


_COMMANDS = {
    "lock": cmd_lock,
    "build": cmd_build,
    "validate": cmd_validate,
    "register": cmd_register,
    "launch": cmd_launch,
    "restart": cmd_restart,
    "watch": cmd_watch,
    "results": cmd_results,
}


def _run(fn: Any, *a: Any, ui: Ui) -> int:
    try:
        return int(fn(*a) or 0)
    except GeneratorError as e:
        ui.error(str(e))
        return e.exit_code
    except KeyboardInterrupt:
        return 130


def scenario_main(sc: Scenario, argv: Sequence[str] | None = None, ui: Ui | None = None) -> int:
    ui = ui or Ui()
    prog = Path(sys.argv[0]).name if sc.source is None else sc.source.name
    args = scenario_parser(f"python {prog}").parse_args(argv)
    return _run(_COMMANDS[args.command], sc, args, ui, ui=ui)


# ----- sortie-gen ----------------------------------------------------------------------------


def load_scenario(path: str | Path, name: str | None = None) -> Scenario:
    """Runs a scenario file (without its ``__main__`` block) and returns its Scenario."""
    p = Path(path)
    if not p.is_file():
        raise GeneratorError(f"no scenario file at {p}")
    module = runpy.run_path(str(p), run_name="__sortie_gen__")
    found = [v for v in module.values() if isinstance(v, Scenario)]
    if name is not None:
        found = [s for s in found if s.name == name]
    unique = list({id(s): s for s in found}.values())
    if not unique:
        raise GeneratorError(f"{p} defines no Scenario" + (f" named {name!r}" if name else ""))
    if len(unique) > 1:
        names = ", ".join(s.name for s in unique)
        raise GeneratorError(f"{p} defines several scenarios ({names}); pick one with --scenario")
    return unique[0]


# Scenario commands that talk to the engine only: `sortie-gen <command>` runs them without a file.
_NO_FILE_NEEDED = ("restart", "watch", "results")


def main_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(prog="sortie-gen", description="Sortie mission generator")
    ap.add_argument("--version", action="version", version=f"sortie-gen {__version__}")
    sub = ap.add_subparsers(dest="command", required=True, metavar="COMMAND")

    p = sub.add_parser("lobby", help="write the lobby mission the server idles in")
    p.add_argument("--out", type=Path, default=Path("missions/sortie-lobby.miz"))
    p.add_argument("--theatre", default="caucasus")

    p = sub.add_parser("worker", help="serve campaign turns to the engine (Generator.Serve)")
    p.add_argument("campaign", type=Path, help="a Python file with next_turn(turn) -> Scenario")
    p.add_argument("--once", action="store_true", help="exit when the stream ends instead of reconnecting")
    _engine_args(p)

    p = sub.add_parser("adopt", help="inspect a mission made in the editor")
    p.add_argument("miz", type=Path)
    p.add_argument("--json", action="store_true")

    for name, help_text in (
        ("lock", "write scenario.lock.json of a scenario file"),
        ("build", "build a scenario file offline"),
        ("validate", "check a scenario file's types with the mission running in DCS"),
        ("register", "build a scenario file and register it with the engine"),
        ("launch", "launch a scenario file's build"),
        ("restart", "reload the mission DCS runs"),
        ("watch", "print live events"),
        ("results", "print mission results"),
    ):
        p = sub.add_parser(name, help=help_text, add_help=False)
        p.add_argument("scenario", type=Path, nargs="?" if name in _NO_FILE_NEEDED else None)
        p.add_argument("rest", nargs=argparse.REMAINDER)
    return ap


def main(argv: Sequence[str] | None = None, ui: Ui | None = None) -> int:
    ui = ui or Ui()
    raw = list(sys.argv[1:] if argv is None else argv)
    # An engine-only command with options and no file: `sortie-gen restart --engine ADDR`.
    if raw and raw[0] in _NO_FILE_NEEDED and (len(raw) == 1 or raw[1].startswith("-")):
        parsed = scenario_parser("sortie-gen").parse_args(raw)
        return _run(_COMMANDS[raw[0]], None, parsed, ui, ui=ui)
    args = main_parser().parse_args(raw)

    if args.command == "lobby":
        from .lobby import build_lobby

        def lobby() -> int:
            try:
                data = build_lobby(args.theatre)
            except ValueError as e:
                raise GeneratorError(str(e)) from None
            atomic_write_bytes(args.out, data)
            ui.say(f"wrote {_rel(args.out)} ({_size(len(data))}, {args.theatre})")
            return 0

        return _run(lobby, ui=ui)

    if args.command == "adopt":
        from .adopt import adopt, render

        def do_adopt() -> int:
            if not args.miz.is_file():
                raise GeneratorError(f"no mission at {args.miz}")
            report = adopt(args.miz)
            ui.say(json.dumps(report.to_json(), indent=2, ensure_ascii=False) if args.json else render(report).rstrip())
            return 0

        return _run(do_adopt, ui=ui)

    if args.command == "worker":
        from .engine import EngineClient
        from .worker import run

        def work() -> int:
            with EngineClient(args.engine, args.token, args.timeout) as client:
                return run(args.campaign, client, once=args.once, log=ui.say)

        return _run(work, ui=ui)

    # Scenario commands on a file.
    rest = list(args.rest)
    if args.command in _NO_FILE_NEEDED and args.scenario is not None and args.scenario.suffix != ".py":
        rest.insert(0, str(args.scenario))
        args.scenario = None
    if args.scenario is None:
        parsed = scenario_parser("sortie-gen").parse_args([args.command, *rest])
        return _run(_COMMANDS[args.command], None, parsed, ui, ui=ui)

    def on_file() -> int:
        sc = load_scenario(args.scenario)
        return scenario_main(sc, [args.command, *rest], ui)

    return _run(on_file, ui=ui)


if __name__ == "__main__":  # pragma: no cover
    sys.exit(main())
