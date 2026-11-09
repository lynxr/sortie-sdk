# Sortie errors and fixes (for agents)

A command error is a `sortie::Error` with a stable code and the Rust call site that built the
command:

```text
error[sortie::gone]: `push_task` on group "Red CAP 1": Unit doesn't exist (at logic/src/lib.rs:48:9)
```

`e.report()` is the long form the runtime logs: the call site, the Lua traceback, the session
and command ids and a `help:` line. `e.code()`, `e.origin()`, `e.lua_traceback()`,
`e.failure()` (op, target, message) give the parts; `e.is_session_ended()` tells the normal
end of a session from a failure.

## Handling errors in a mission

```rust
let sam = match ctx.group(groups::SAM_NORTH) {
    Ok(g) => g,
    // In the manifest, but no live unit now (late activation, already destroyed).
    Err(sortie::Error::NotInWorld { .. }) => return Ok(()),
    // UnknownSymbol: the running mission was built from another scenario lock.
    Err(e) => return Err(e),
};
match sam.set_option(Alarm::Red).key("sam-hot").await {
    Ok(()) => {}
    // The group died between the check and the command: expected in a live mission.
    Err(sortie::Error::Gone(f)) => eprintln!("SAM North is gone: {}", f.message),
    // The session ended while the command was in flight: not a logic error.
    Err(e) if e.is_session_ended() => return Ok(()),
    // Anything else: the runtime records it with `e.report()` (code, call site, hint).
    Err(e) => return Err(e),
}
```

An `Err` that leaves the mission function or a task spawned with `ctx.spawn` is recorded as a
logic error (stderr of the logic, with its call site) and ends that task only. `SessionEnded`
is never recorded. A panic is recorded the same way, with its location.

## Command errors

| Code | Meaning | Usual cause → fix |
| --- | --- | --- |
| `sortie::gone` | the target no longer exists in DCS | it died or was despawned before the command ran → match `Error::Gone` where a target can die (always in a live mission) |
| `sortie::timeout` | DCS did not answer in time (10 s, 30 s for spawns and destroys) | a big spawn wave queued behind the spawn pacing, or a busy server → `.timeout(..)`, spawn in smaller steps, check `sortie status` / `sortie top` |
| `sortie::lua_error` | the Lua layer raised an error | a bad `Task::raw` table, a wrong option for the unit type → read `e.lua_traceback()` |
| `sortie::invalid` | the command does not fit | a spawn or destroy without `.key(..)` under `--dev`; a key reused for another command ("key `k` was already used for a different command in session ..."); a manifest without the template |
| `sortie::name_conflict` | a live group already has this name | `.named("..")` twice → drop `.named` (default names are unique) or add a counter |
| `sortie::session_ended` | the session ended before the command completed | normal at the end of a turn → return `Ok(())` |
| `sortie::unavailable` | the engine cannot run it now | no mission session is running; or a keyed command was in flight when the DCS link dropped ("whether it ran is not known": check the world, then use a new key) |
| `sortie::paused` | the simulation is paused | the command was sent with `.on_pause(v1::PausePolicy::Fail)` |
| `sortie::rejected` | the engine refused the command | read the message |
| `sortie::quarantined` | this command kind is disabled after it faulted the bridge | run `sortie doctor`, report it with `Logs/sortie-bridge.log` |
| `sortie::bridge_fault` | the bridge caught a fault | report it with `Logs/sortie-bridge.log` |
| `sortie::conflict` | a state write lost against another writer | `StateDoc::update` retries by itself; seen only after 8 conflicts in a row |

## Lookup and state errors

| Code | Message | Fix |
| --- | --- | --- |
| `sortie::unknown_symbol` | "the running mission has no group `X`; regenerate the scenario lock" | the mission in DCS was built from another lock: `python scenario.py lock`, `cargo build`, register and launch the mission again |
| `sortie::not_in_world` | "group `X` is not in the world right now" | late activation, or all its units are dead → check `w.group(&groups::X).alive_count()` first, or match the error |
| `sortie::state` | "state `name`: ..." | the engine's store refused: read the message; a `T` that no longer deserializes the stored JSON (changed field types) also lands here: add `#[serde(default)]` to new fields |
| `sortie::transport` | "engine connection: ..." | the engine is unreachable: start `sortied`, check `--engine` / `SORTIE_ENGINE` |

## Build errors

| Where | Message (start) | Fix |
| --- | --- | --- |
| `build.rs` | `error: sortie-build failed for scenario "x": <path>: ...` + `help: the path is relative to the logic crate's Cargo.toml; ...` | the lock path is wrong or the lock is missing: `python scenario.py lock` |
| `build.rs` | `... is for scenario "a", but build.rs asked for "b"` | the first argument of `sortie_build::scenario(..)` must be the lock's scenario name |
| `build.rs` | `help: the lock was edited by hand or is corrupt; ...` | regenerate the lock; never edit it |
| `include_scenario!` | `OUT_DIR is not set, so there is no generated scenario module to include` | add `sortie-build` to `[build-dependencies]` and the one-line `build.rs` |
| `include_scenario!` | ``no generated module for scenario `x` (...); build.rs generated: ...`` | use the same name in `include_scenario!` and `sortie_build::scenario` |
| `#[sortie::mission]` | ``a mission must be an `async fn` ...``, `the arguments are swapped ...`, ``a mission must return `sortie::Result<()>` ...`` | the signature is `async fn run(ctx: Ctx<M>, start: Start) -> sortie::Result<()>` |
| `#[sortie::mission]` | `` `X` is not a scenario marker `` | `Ctx<M>` takes the marker from the generated module, e.g. `Ctx<frontline::Frontline>` |
| `#[sortie::test]` | a compile error: tests without DCS arrive in SDK 0.2 | test decisions in plain functions with `#[test]`; test the mission in DCS |
| any `.await` on a borrow | clippy `await_holding_refcell_ref` | end the `ctx.world()` / `StateDoc::get()` borrow before the `.await` |

## Attach and runtime messages (the logic's stderr)

| Message | Meaning → fix |
| --- | --- |
| ``cannot reach the engine at http://127.0.0.1:50777: ... (is `sortied` running?); retrying in N s`` | start `sortied`; the logic keeps retrying (1 to 30 s) |
| ``error[sortie::transport]: Logic.Attach at ... failed: ... (set SORTIE_TOKEN to a token with the `logic` role)`` | exit code 2: set `SORTIE_TOKEN` (from `Saved Games\DCS\Sortie\tokens.toml`), or run `sortied --dev` locally |
| ``error[sortie::rejected]: the engine rejected this logic: logic `x` was built against a different `s` schema: N symbol(s) missing from the running mission`` + `missing symbols: ...` | exit code 2: the mission in DCS lacks symbols this logic was built with; build the logic and the mission from the same lock |
| ``another PRIMARY logic is attached for `s` `` | a second copy of the logic is running: stop the old process |
| ``a `observer` token cannot attach a logic (needs `logic` or `admin`)`` | use a `logic` token |
| `` `spawn_group` without `.key(..)` in dev mode: a hot reload would run it again; add a key such as `.key("spawn-group-1")` `` | the logic runs with `--dev`: add a key |
| ``in dev mode spawn_group and destroy need `.key(..)` (hot reload replays commands by key)`` | `sortied --dev` refused it: add a key |
| ``tick N: the world replica is borrowed across an `.await` ...`` | a `ctx.world()` guard lives across `.await`: shorten the borrow |
| `event stream lagged` (warning, `skipped = N`) | an event loop is too slow: keep it short, move slow work to another task |
| `task spawned at src/lib.rs:L:C returned` + report, or `... panicked at ...` | that task ended with an error; the others keep running |
