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
- **All game logic is here.** The `.miz` holds the map side only; what happens in the
  mission is this Rust code, never a DO SCRIPT in the `.miz` (rule 1).

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
sortie = { git = "https://github.com/lynxr/sortie-sdk", tag = "v0.4.0" }
serde = { version = "1", features = ["derive"] }   # only for state documents

[build-dependencies]
sortie-build = { git = "https://github.com/lynxr/sortie-sdk", tag = "v0.4.0" }

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
   sortie-live-debugging skill). The public SDK has no tests without DCS: FakeDcs carries
   the engine and is not part of it yet, so `#[sortie::test]` is a compile error. Put pure
   decisions (which wave, which target) in plain functions and test them with ordinary
   `#[test]`.

## Rules

1. **Game logic lives in the Rust logic, never in the `.miz`.** The `.miz` that
   `scenario.py` builds holds the map side only (groups, units, statics, zones, weather,
   briefing, the warehouses' starting stock) and Sortie's bootstrap. What happens when,
   spawns, tasks, scoring, menus, marks and stock changes are all Rust. When the SDK lacks
   something, in this order:
   - a typed command or query (the API map below);
   - a catalog call: its generated wrapper (`ctx.dcs().land().height(p)`); with the cargo
     feature `dcs-raw` also by path (`ctx.dcs().call("land.getHeight")..`) and a batch of
     many calls in one frame (`ctx.dcs().batch()`);
   - a raw DCS task table, `group.set_task(Task::raw(table))`: any group takes it, for what
     the typed tasks lack (a stop, tasks at waypoints, a ship's route: references/api.md);
   - tell the Sortie team what is missing.
   A DO SCRIPT added through a `@sc.pydcs` hook is only for a third-party script the
   mission must keep (Sortie does not check it), never for the mission's own logic. The
   build warns about every such script (also an init script or a LUA PREDICATE), and the
   template's `scenario.py` sets `do_scripts="deny"`, which fails the build.
2. **Key every command whose repetition matters.** `spawn_group` and `destroy` always: they
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
3. **Wait on states, not events,** when the condition is visible in the world:
   `ctx.wait_until(|w| w.group(&sam).alive_count() == 0)`. A replay sees the state; it never
   sees an event that happened before it subscribed.
4. **Delays that must survive a logic restart are named timers:**
   `ctx.timer("cap-delay", mins(10))`. `ctx.sleep` starts over after a restart.
5. **Campaign changes go inside `ctx.once_per_turn`.** After a mission restart the turn is
   played again in a new session; `once_per_turn` keeps a kill from being counted twice in one
   turn. `ctx.once` is per session.
6. **Short borrows.** Take `ctx.world()` and `StateDoc::get()` in a statement or a block that
   ends before the next `.await`. A borrow held across `.await` stops the replica from updating
   (the runtime records "the world replica is borrowed across an `.await`").
7. **Never block the thread.** No `std::thread::sleep`, blocking I/O or long CPU loops: ticks
   stop, and after 4096 unread messages the engine detaches the logic (it exits with code 0).
   Wait with `ctx.sleep`, `ctx.timer`, `ctx.wait_until`.
8. **Background work is `ctx.spawn`.** Its tasks end with the session; an `Err` or a panic in
   one is recorded with its call site and the other tasks keep running. No `std::thread`, no
   runtime of your own.

## Start kinds

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
| a scenario group | `ctx.group(groups::X)?` → `Group<Ground/Air/Helo/Ship>`; `set_option(Alarm::Red / Roe::..)`, `destroy()`, `push_task`/`set_task` (`Task::cap/orbit/rtb` for air and helicopters; `Task::ground_route(..)` for ground groups, API revision 3; a raw DCS task table, `Task::raw(..)`, for any group) |
| spawn | `ctx.spawn_group(templates::X)` or `templates::X.named("..")`, then `.from_airbase(airbases::Y, StartType::..)`, `.at(Vec3, heading_rad)` (every unit faces `heading_rad`, the formation is not turned) or `.at_formation(Vec3, heading_rad)` (API revision 3: the formation turns as one body, each unit keeps its heading offset), `.key(..)` → `Group<K>` |
| tell players | `ctx.message(Coalition::Blue / &group / press.group / sortie::MessageTarget::All).text(..).secs(..)` |
| read the world | `ctx.world()`, `w.units().coalition(..).air().in_zone(zones::X).count()`, `w.players()`, `w.group(&g).alive_count()` |
| wait | `ctx.wait_until(pred)`, `ctx.timer(name, dur)`, `ctx.sleep(dur)`, `ctx.until_session_end()` |
| react | `ctx.events().of::<v1::Dead>().in_group(&g)`; `use sortie::proto::v1;` |
| F10 menu | `ctx.menu(Coalition::Blue).per_group().command(menus::X).key(..).await?`, then `.presses()` |
| F10 menu tree (one group, sub-menus) | `ctx.menus().tree("jtac").to(Audience::group(..)).command(id, label).submenu(label, \|s\| ..).await?`, then `.presses()`; needs `#[sortie::mission(api_rev = 1, event_rev = 1)]`; a level of more than 10 entries pages itself (`More...`; on the tree's root level `More... (<tree name>)`) unless `.overflow(Overflow::Reject)` |
| flags | `ctx.flag(flags::X).set(true)`, `.get()` → `i32` |
| remember | `ctx.state::<T>("name")` (campaign), `ctx.state_in::<T>(v1::StateScope::Turn, ..)`, `ctx.once`, `ctx.once_per_turn` |
| results | `ctx.report().set("key", value)`, `ctx.end_session(Outcome::BlueWin).text(..)` |
| data for a web site (live map, campaign page) | `ctx.publish("front", &value).await?` (any `Serialize`; the engine serves it at `GET /api/v1/docs/<logic>/front`), `ctx.unpublish(name)`; needs `#[sortie::mission(api_rev = 3)]` |
| a web site asks the logic to do something | `let mut reqs = ctx.api().action("send_cap")?;` then `while let Some(req) = reqs.next().await { .. req.reply(&value)? / req.fail(422, why)? }` (called as `POST /api/v1/actions/<logic>/send_cap` with an `operator` token; answer within 10 s; a dropped request is 500); needs `#[sortie::mission(api_rev = 3)]` |
| airbase owner, auto capture, radio silence | `ctx.airbase_control([airbases::X, airbases::Y]).owner(Coalition::Red).auto_capture(false).radio_silent(true).key(..)` (1..=32 bases, one frame) → per-base `before` / `after` / `error()`; the world has the owner at once; needs `#[sortie::mission(api_rev = 3)]` |
| airbase owner changed (any cause) | `ctx.events().of::<v1::AirbaseOwnerChanged>()`, `.airbase(..)`, `.cause(v1::OwnerChangeCause::Captured)`; one per change (DCS capture, `airbase_control`, a read of DCS); an owner another script changed without an event comes only from a read: poll `ctx.airbases_state(..)`; needs `#[sortie::mission(api_rev = 3, event_rev = 2)]` |
| airbase parking, runways, control state | `ctx.airbase(airbases::X).parking().available_only()` → `Vec<ParkingSpot>`, `.runways()` → `Vec<Runway>`, `.state()` → `AirbaseState`; many: `ctx.airbases_parking([..])` / `airbases_runways` / `airbases_state` → per-base `value` / `error()`; reads, no key, API revision 3 |
| warehouse stock | `ctx.warehouse(airbases::X).stock()` → `WarehouseStock { aircraft, weapons, liquids }` (`StockClass::Unlimited` or `Limited(name -> n)`); `.items([..])` / `.liquids()` count only those; many: `ctx.warehouses_stock([..])` → per-warehouse `value` / `error()`; a read, no key, API revision 3 |
| change warehouse stock, move it between warehouses | `ctx.warehouse_op().transfer(airbases::X, airbases::Y, "weapons.missiles.AIM_120C", 6.0).add(airbases::Y, Liquid::JetFuel, 5000.0).key(..)` (also `.set`, `.remove`; 1..=16 changes, one frame, all or nothing) → `StockChanges { applied, changes }` (per change `outcome`: `Done`, `NotEnough`, `Unlimited`, `Aliased`, ...); every warehouse_op needs a key in dev, sets too; name an item in one form per command; API revision 3 |
| the names DCS gives warehouse items, name <-> wsType (check a campaign's items before stocking; no `Warehouse.getResourceMap` catalog entry) | `ctx.item_names()` → `ItemNames` (`.ws_type_of("weapons.missiles.AIM_120C")`, `.names_of([4, 4, 7, 106])`, `.knows(..)`); `.items([..])` looks up only those (`unknown` for the rest); read from the running DCS once per DCS process (`cached` after); a read that takes no key (`.key(..)` is `Error::Invalid`), API revision 3 |
| a DCS function with no typed operation | `ctx.dcs().land().surface_type(p)`, `ctx.dcs().airbase(airbases::X).callsign()`, `ctx.dcs().sounds().out_sound(..)`; needs `#[sortie::mission(api_rev = 1)]` |
| a ground group drives somewhere (a convoy, a route along roads) | `convoy.set_task(Task::ground_route([RoutePoint::on_road(p), ..]))` (API revision 3); a stop or what the route lacks: `Task::raw(..)`; references/api.md, "A ground group's route" |
| the road between two map points (a convoy's distance, a stop on the way) | `ctx.dcs().land().path_on_roads("roads", from.north, from.east, to.north, to.east)` → `Option<Vec<MapPoint>>`, start to end; class heavy (one at a time, 2 per second, its own DCS-time budget, ends at most 300 km apart, never in a batch); needs `#[sortie::mission(api_rev = 3)]`: references/api.md, "A road path" |
| the length and ends of a road path without its points; a few of its turning points for a route | `ctx.dcs().land().road_path("roads", from, to, Some(8.0), None)` → `Option<RoadPath>` (`count`, `length_m`, `first`/`last`, `start_gap_m`/`end_gap_m`, `points` capped with both ends, `thinned`), summarised by the Lua layer inside DCS; class heavy; needs `#[sortie::mission(api_rev = 3)]`: references/api.md, "The summary of a road path" |

Signatures, defaults and an example for each: [references/api.md](references/api.md).

## More

- [references/patterns.md](references/patterns.md): a long multiplayer mission (thousands of
  units, players joining and leaving, AI waves, polling, scores) and the anti-patterns.
- [references/errors.md](references/errors.md): every `sortie::*` error code, the build and
  attach errors, and their fixes.
- Docs (Russian): https://lynxr.github.io/sortie-sdk/. Scenario side: the sortie-scenario skill. Running in DCS:
  the sortie-live-debugging skill.
