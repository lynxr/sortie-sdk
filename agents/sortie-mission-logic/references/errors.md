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

<!-- snippet: skills-errors-match -->
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
is never recorded. A panic is recorded the same way, with its location. Nothing runs the
mission function again until the next session or a restart of the logic, so the work after
the failed command never happens: do not end the mission function on one optional command (an
AI option, a message, smoke). Give it a key, retry it once on `Timeout`, log the rest:

<!-- snippet: skills-errors-optional -->
```rust
let alarm = || sam.set_option(Alarm::Red).key("sam-north-red");
let mut done = alarm().await;
if let Err(sortie::Error::Timeout(_)) = &done {
    // The same key: a first try that reached DCS after all does not run twice.
    done = alarm().await;
}
if let Err(e) = done {
    // Optional: note it and go on instead of ending the mission function.
    eprintln!("SAM North stays as it was: {}", e.report());
}
```

## Limits (engine 0.4.0)

A refusal by one of the engine's limits is told by data, never by its text: `e.limit()` ->
`Option<&sortie::Limit>` (`kind: LimitKind`, `budget`, `writer`, `capacity`, `per_second`,
`needed`, `available`), `e.retry_after()` -> `Option<Duration>`, `e.is_limit()`. The code
and `e.capability()` are what 0.3.0 gave; against a 0.3.0 engine both are `None`. The SDK
never retries by itself.

<!-- snippet: skills-errors-limit -->
```rust
let mut tries = 0;
let fuel = loop {
    match ctx.dcs().unit(UnitSel::name("SAM North-1")).fuel().await {
        Ok(f) => break Some(f),
        // A rate or DCS-time budget: the engine names the wait of this same read.
        Err(e) if tries < 3 && e.retry_after().is_some() => {
            tries += 1;
            ctx.sleep(e.retry_after().unwrap_or_default()).await;
        }
        // A forbid (not in the catalog, switched off): waiting never helps.
        Err(e) if e.is_forbidden() => break None,
        Err(e) => return Err(e),
    }
};
```

| `LimitKind` | `budget` | Error | What helps |
| --- | --- | --- | --- |
| `CallRate` | `call_rate.read` (200/s), `.mutate` (50/s), `.heavy` (2/s), `.admin` (1/s) | `Rejected` | wait `retry_after()` |
| `DcsTime` | `dcs_time.read` (reads and batches; a single read sees the debt floored at -20 ms, a batch the whole debt), `dcs_time.heavy` (heavy calls): 20 ms of DCS time a second each | `Rejected` | wait `retry_after()`; `None` for a heavy call sent while the logic's previous one is not back: send it after that one answered |
| `BatchSize`, `RequestSize` | `batch.calls` (64, 256 reads), `request.bytes` (64 KiB) | `Invalid` | split the batch or request |
| `CreatedHandles` | `handles.created` (256 alive) | `Rejected` (capability kept) | destroy one first |
| `ReadHandles` | `handles.read` (1024, least recently used dropped) | `Gone` | read the object again |
| `SlowCalls` | `bridge.slow_calls` (3 calls over 1 s in 10 min) | `Quarantined` | nothing until DCS restarts |
| waits: `TaskingDelay`, `HeavySlot`, `LinkCredits`, `LinkDown`, `After`, `Queue`, `BridgeDeadline`, `SpawnGate`, `SpawnWindow` | `delay.tasking`, `slot.heavy`, ... | `Timeout` | it never ran in DCS: a retry is safe |
| `After` when the `.after(..)` command failed | `wait.after` | that command's code and capability (`Rejected`, `Invalid`, ...) | it never ran; sent again it fails the same way at once: send the failed one again, then this one `.after(..)` the new one |

- `retry_after()` is the wait of **the same command sent again unchanged**, from the refusal,
  if the logic sends nothing else meanwhile: after a 256-read batch from a full bucket (200 -
  256 = -56) another 256-read batch waits 1.28 s, a single read 0.285 s. When several budgets
  refuse, the limit is the one with the longest wait. `None` for sizes, caps and waits, and for
  a heavy call refused while the logic's previous heavy call is not back (one heavy call of a
  writer at a time: do not `join!` two).
- A batch refused at the limit check ran no call (the engine checks it whole before sending).
- `e.is_forbidden()`: `Rejected` with a capability and no limit (a path not in the catalog, off
  by default or switched off by the owner, an operation disabled in `engine.toml`).
- `ctx.sleep` follows the ticks, so a wait can end a little early: the next refusal names the
  rest, which the loop above survives.

## Command errors

| Code | Meaning | Usual cause → fix |
| --- | --- | --- |
| `sortie::gone` | the target no longer exists in DCS | it died or was despawned before the command ran → match `Error::Gone` where a target can die (always in a live mission) |
| `sortie::timeout` | the deadline passed (10 s, 30 s for spawns and destroys). Two kinds: "the deadline passed before the command could start (why)" never reached DCS (`e.limit()` names what it waited for: `TaskingDelay`, `HeavySlot`, `LinkCredits`, `LinkDown`, `After`, `BridgeDeadline`, `Queue`); "`op` was sent to DCS but no result arrived before the deadline" may still run (`e.limit()` is `None`) | never reached DCS: retry (safe); sent: retry only with the same `.key(..)`, which waits for the late result instead of running it twice. A task or AI option right after a spawn waits 2 frames and 0.5 s of model time (`set_task`, `push_task`, `set_option`; held without its deadline running while the spawn is still queued or in DCS since 0.4.0); a busy server → `.timeout(..)`, check `sortie status` / `sortie top` |
| `sortie::lua_error` | the Lua layer raised an error | a bad `Task::raw` table, a wrong option for the unit type → read `e.lua_traceback()` |
| `sortie::invalid` | the command does not fit | a spawn or destroy without `.key(..)` under `--dev`; a key reused for another command ("key `k` was already used for a different command in session ...", also after a logic restart: a changed command needs a new key such as `sam-red-v2`); a batch or request over its size (`e.limit()`: `BatchSize`, `RequestSize`); a manifest without the template; `Task::rtb` (or a `Task` converted from a CAP or orbit) for a ground group ("group `X` is not an aircraft group; only a ground route or a raw task applies to it": give it `Task::ground_route(..)`, or `Task::raw` for what the route lacks, such as a stop) or for a naval group ("...; only raw tasks apply to it": give it `Task::raw`); a ground route that does not fit: through `Task::from(..)` for a group that is no ground group ("a ground route is for ground groups"), with no point, with more than 256 points ("a ground route takes at most 256 points, got N": give the places where it turns or stops, not every point of a road path), a speed not above 0 |
| `sortie::name_conflict` | a live group already has this name | `.named("..")` twice → drop `.named` (default names are unique) or add a counter |
| `sortie::session_ended` | the session ended before the command completed | normal at the end of a turn → return `Ok(())` |
| `sortie::unavailable` | the engine cannot run it now | no mission session is running; or a keyed command was in flight when the DCS link dropped ("whether it ran is not known": check the world, then use a new key) |
| `sortie::paused` | the simulation is paused | the command was sent with `.on_pause(v1::PausePolicy::Fail)` |
| `sortie::rejected` | the engine refused the command | a limit (`e.limit()` is `Some`: a rate, the DCS time, the created handles) → wait `e.retry_after()` and send it again (section "Limits"); a forbid (`e.is_forbidden()`) → no retry helps; read the message; capability `dev_eval` = a raw table (`Task::raw`, `.raw(..)` of a static, a `ctx.dcs()` argument) holds Lua code or a file name at the path the message names (`Script` / `ScriptFile` actions, `condition`, `command`, `file` strings) → use the typed operation; raw tables carry data only; another capability (`e.capability()`) = the engine owner switched that operation off (`engine.toml [operations]`) |
| `sortie::quarantined` | this command kind (for a universal call: this catalog function) is quarantined: DCS crashed while one ran, or it faulted the bridge 3 times | `e.capability()` names it (`"call:mission:land.getHeight"`: one catalog function); the engine owner releases it with `sortie caps --release <name>`; report it with `Logs/sortie-bridge.log` |
| `sortie::bridge_fault` | the bridge caught a fault | report it with `Logs/sortie-bridge.log` |
| `sortie::conflict` | a state write lost against another writer | `StateDoc::update` retries by itself; seen only after 8 conflicts in a row |
| `sortie::result_not_kept` | a keyed `ctx.dcs()` call was answered from the key store, which kept no values for it (a result above 64 KiB); the call ran before and was not run again | read what you need with a call of its own; results up to 64 KiB, handles and objects included, come back on a repeat |
| `sortie::revision` | the operation belongs to a newer API revision than the logic declares; nothing was sent | declare it: `#[sortie::mission(api_rev = 1)]` (revisions are written in the logic's source, default 0) |
| `sortie::engine_too_old` | the engine's API revision (`ctx.engine_api_rev()`) is below the operation's; nothing was sent | update `sortied`, or keep to the operations of the engine's revision |

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
| `#[sortie::test]` | a compile error: FakeDcs carries the engine and is not part of the public SDK yet | test decisions in plain functions with `#[test]`; test the mission in DCS |
| `set_task` / `push_task` | ``error[E0277]: `Ground` groups do not fly: CAP and orbit are tasks for aircraft and helicopters`` | a ground group takes a route, `Task::ground_route(..)`, or a raw DCS task table, `Task::raw(..)`; a naval group a raw table (references/api.md, "A ground group's route") |
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
