---
name: sortie-mission-logic
description: Sortie mission logic in Rust (crate `sortie`, DCS World). Use when writing or changing a `#[sortie::mission]` or code that uses `Ctx` (commands, `.key()`, `ctx.once`, waits, events, F10 menus, flags, state, timers, `end_session`), when handling `Start` kinds, when a `sortie::*` error code appears (`sortie::gone`, `sortie::invalid`, `sortie::unknown_symbol`, ...), or when designing a long multiplayer mission. Applies to any crate that depends on `sortie` or calls `sortie::include_scenario!`.
---

# Sortie mission logic

The mission logic is an ordinary async Rust program next to DCS. The engine `sortied` owns
the link to DCS; the logic attaches to it over gRPC and gets one call of its mission function
per session.

- **Reads are local and synchronous.** `ctx.world()` is a replica of the DCS world, updated
  10 times a second and by every event. No RPC.
- **Writes are commands.** Every write is a `Cmd` builder; `.await` sends it and resolves when
  DCS has executed it (or with a `sortie::Error`).
- **The mission function is a replayable script.** It runs again from the top when the logic
  process restarts inside a running session (`Start::AfterCrash`). Every step must be safe to
  run twice: keyed commands, `ctx.once`, named timers, waits on world state.
- **One thread.** `Ctx` is cheap to clone and not `Send`; background work is `ctx.spawn`.
- **Typed scenario.** `groups::X`, `zones::X`, `templates::X`, `flags::X`, `airbases::X`,
  `menus::X` come from the scenario lock; a symbol of another scenario does not compile.

## Project layout

A mission repository starts as a copy of the SDK's `template/` folder:

```text
my-mission/
  Cargo.toml             # the logic crate (package my_mission_logic)
  build.rs               # turns the lock into typed symbols
  src/lib.rs             # include_scenario! and the #[sortie::mission]
  src/main.rs            # one line: sortie::main!(my_mission_logic::run);
  scenario.py            # the scenario (sortie-gen); see the sortie-scenario skill
  scenario.lock.json     # written by `python scenario.py lock`; commit it, never edit it
  pyproject.toml         # uv project with the sortie-gen dependency
```

`examples/op_north` keeps the crate in a `logic/` subfolder instead; both work.

```toml
# Cargo.toml
[package]
name = "my_mission_logic"
version = "0.1.0"
edition = "2024"
rust-version = "1.89"

[dependencies]
sortie = { git = "https://github.com/lynxr/sortie-sdk", tag = "v0.1.0" }
serde = { version = "1", features = ["derive"] }   # only for state documents

[build-dependencies]
sortie-build = { git = "https://github.com/lynxr/sortie-sdk", tag = "v0.1.0" }

[lints.clippy]
await_holding_refcell_ref = "deny"   # catches a world borrow held across .await
```

- `build.rs`: `fn main() { sortie_build::scenario("my_mission", "scenario.lock.json").emit(); }`.
  The first argument is the scenario name; the path is relative to the crate's `Cargo.toml`.
- `src/lib.rs` starts with `sortie::include_scenario!(my_mission);`, which defines
  `pub mod my_mission` with the marker type `MyMission` and the symbol modules.
- `src/main.rs` is `sortie::main!(my_mission_logic::run);`: arguments, attach, reconnects.
- A private SDK repository needs credentials for cargo (`net.git-fetch-with-cli = true` in
  `.cargo/config.toml`, plus your git credential helper) and for uv: see the docs.

## The mission function

The examples in this skill use the `frontline` scenario of the sortie-scenario skill.

<!-- snippet: skills-logic-skeleton -->
```rust
use sortie::prelude::*;

// lib.rs of the logic crate also has `sortie::include_scenario!(frontline);`
use frontline::{Frontline, groups, zones};

#[sortie::mission] // runs once per session of `frontline`
pub async fn run(ctx: Ctx<Frontline>, start: Start) -> sortie::Result<()> {
    let sam = ctx.group(groups::SAM_NORTH)?; // Group<Ground>, checked against the manifest
    sam.set_option(Alarm::Green).key("sam-dark").await?;

    if start.is_fresh() {
        ctx.message(Coalition::Blue)
            .text("SAM North is cold. Push when ready.")
            .secs(20)
            .key("intro")
            .await?;
    }

    // Evaluated on the local replica after every tick and every event: no RPC.
    ctx.wait_until(|w| {
        w.units()
            .coalition(Coalition::Blue)
            .air()
            .any_inside(zones::SAM_RING)
    })
    .await;
    sam.set_option(Alarm::Red).key("sam-hot").await?;

    // A state, not an event: still correct when the function runs again after a crash.
    ctx.wait_until(|w| w.group(&sam).alive_count() == 0).await;
    ctx.end_session(Outcome::BlueWin)
        .text("SAM North is down")
        .await
}
```

`#[sortie::mission]` checks the signature: `async fn`, `ctx: Ctx<M>` first, `start: Start`
second, return type `sortie::Result<()>`. Returning `Err` records a logic error with the call
site; the session goes on.

## Workflow

1. **Symbols first.** Everything the logic names (a group, zone, template, flag, airbase or
   menu) needs an `ident=` in `scenario.py`. Add it there, run `python scenario.py lock`, then
   build the logic. Groups added only by a `@sc.pydcs` hook have no symbol: reach them by DCS
   name in world queries (`w.group("Name")`, `.in_group("Name")`).
2. **Write the step replay-safe** (rules below), then `cargo build` and `cargo clippy`.
   Done when both pass and every `spawn_group` and `destroy` in the diff carries `.key(..)`.
3. **Run it in DCS** with `sortied --dev` and the logic's `--dev` (see the
   sortie-live-debugging skill). SDK 0.1.0 has no tests without DCS: FakeDcs
   arrives in SDK 0.2, and `#[sortie::test]` is a compile error until then. Put pure decisions
   (which wave, which target) in plain functions and test them with ordinary `#[test]`.

## Rules

1. **Key every command whose repetition matters.** `spawn_group` and `destroy` always: they
   are refused without a key when `sortied` runs with `--dev` and when the logic runs with
   `--dev`. Key messages, smoke, flags and menu commands that a replay must not repeat. Key
   semantics:
   - the scope is the session: within it, a key that already ran returns the stored result
     without reaching DCS (a replayed spawn returns the same `Group`);
   - a key names a step, so it is the same on every run: a literal (`"red-cap-1"`) or built
     from a deterministic counter (`format!("wave-{n}")`), never from model time, a random
     number or an Eid;
   - one key, one command: reusing a key for a different body (other text, other target) fails
     with `sortie::invalid` ("key `k` was already used for a different command in session ...");
   - a keyed command that timed out after reaching DCS is not sent again on retry with the
     same key: the retry waits for the late result;
   - a mission restart is a new session: keys start empty again.
2. **Wait on states, not events,** when the condition is visible in the world:
   `ctx.wait_until(|w| w.group(&sam).alive_count() == 0)`. A replay sees the state; it never
   sees an event that happened before it subscribed.
3. **Delays that must survive a logic restart are named timers:**
   `ctx.timer("cap-delay", mins(10))`. `ctx.sleep` starts over after a restart.
4. **Campaign changes go inside `ctx.once_per_turn`.** After a mission restart the turn is
   played again in a new session; `once_per_turn` keeps a kill from being counted twice in one
   turn. `ctx.once` is per session.
5. **Short borrows.** Take `ctx.world()` and `StateDoc::get()` in a statement or a block that
   ends before the next `.await`. A borrow held across `.await` stops the replica from updating
   (the runtime records "the world replica is borrowed across an `.await`").
6. **Never block the thread.** No `std::thread::sleep`, blocking I/O or long CPU loops: ticks
   stop, and after 4096 unread messages the engine detaches the logic (it exits with code 0).
   Wait with `ctx.sleep`, `ctx.timer`, `ctx.wait_until`.
7. **Background work is `ctx.spawn`.** Its tasks end with the session; an `Err` or a panic in
   one is recorded with its call site and the other tasks keep running. No `std::thread`, no
   runtime of your own.

## Start kinds

<!-- snippet: skills-logic-start-kinds -->
```rust
match start {
    // First session of this build: the world is as the manifest describes it.
    Start::Fresh => {}
    // The mission was reloaded (same turn): a fresh world, fresh keys and `once`s;
    // `once_per_turn` and the campaign state remember the earlier session.
    Start::Restart => {
        ctx.message(Coalition::Blue)
            .text("Mission restarted")
            .key("restarted")
            .await?;
    }
    // The logic process restarted inside a running session (crash, or you restarted it
    // after a rebuild): the world is as it is now; keys, `once` and timers remember.
    // `Resumed` (the engine resumed the session after its own restart) is not sent yet;
    // treat it the same way.
    Start::AfterCrash | Start::Resumed => {}
}
```

| `Start` | When | The world | Keys, `once`, timers | `once_per_turn`, campaign state |
| --- | --- | --- | --- | --- |
| `Fresh` | first session of this build (turn) | as in the manifest | empty | as stored |
| `Restart` | the mission was reloaded (`scenario.py restart`, or the same build loaded again) | as in the manifest | empty (new session) | remembered |
| `AfterCrash` | the logic process restarted in the running session (crash, or restarted after a rebuild) | as it is now | remembered | remembered |
| `Resumed` | the engine resumed a session after its own restart (not sent yet) | as it is now | remembered | remembered |

A replay-safe mission needs no branch on `Start` except for player-facing extras (a briefing on
`Fresh`, a notice on `Restart`).

## API map

| Need | Use |
| --- | --- |
| a scenario group | `ctx.group(groups::X)?` → `Group<Ground/Air/Helo/Ship>`; `set_option(Alarm::Red / Roe::..)`, `destroy()`, `push_task`/`set_task` (air and helicopters) |
| spawn | `ctx.spawn_group(templates::X)` or `templates::X.named("..")`, then `.from_airbase(airbases::Y, StartType::..)` or `.at(Vec3, heading_rad)`, `.key(..)` → `Group<K>` |
| tell players | `ctx.message(Coalition::Blue / &group / press.group / sortie::MessageTarget::All).text(..).secs(..)` |
| read the world | `ctx.world()`, `w.units().coalition(..).air().in_zone(zones::X).count()`, `w.players()`, `w.group(&g).alive_count()` |
| wait | `ctx.wait_until(pred)`, `ctx.timer(name, dur)`, `ctx.sleep(dur)`, `ctx.until_session_end()` |
| react | `ctx.events().of::<v1::Dead>().in_group(&g)`; `use sortie::proto::v1;` |
| F10 menu | `ctx.menu(Coalition::Blue).per_group().command(menus::X).key(..).await?`, then `.presses()` |
| flags | `ctx.flag(flags::X).set(true)`, `.get()` → `i32` |
| remember | `ctx.state::<T>("name")` (campaign), `ctx.state_in::<T>(v1::StateScope::Turn, ..)`, `ctx.once`, `ctx.once_per_turn` |
| results | `ctx.report().set("key", value)`, `ctx.end_session(Outcome::BlueWin).text(..)` |

Signatures, defaults and an example for each: [references/api.md](references/api.md).

## More

- [references/patterns.md](references/patterns.md): a long multiplayer mission (thousands of
  units, players joining and leaving, AI waves, polling, scores) and the anti-patterns.
- [references/errors.md](references/errors.md): every `sortie::*` error code, the build and
  attach errors, and their fixes.
- Docs (Russian): https://lynxr.github.io/sortie-sdk/. Scenario side: the sortie-scenario skill. Running in DCS:
  the sortie-live-debugging skill.
