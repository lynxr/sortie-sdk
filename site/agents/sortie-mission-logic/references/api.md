# Sortie SDK API reference (for agents)

Everything below is in `sortie::prelude::*` unless a path is given. Event and state enums live
in `sortie::proto::v1` (`use sortie::proto::v1;`). The examples use the `frontline` scenario
(module `frontline`, marker `Frontline`).

## Commands

Every write returns a `Cmd<..>`. Nothing happens until it is awaited; `.await` gives
`sortie::Result<Output>`.

```rust
ctx.smoke(zones::OBJECTIVE.center(), SmokeColor::Red)
    .key("smoke-obj")
    .await?;
ctx.flag(flags::SAM_DOWN)
    .set(true)
    .key("flag-sam-down")
    .await?;

let cap = ctx
    .spawn_group(templates::RED_CAP.named("Red CAP 1"))
    .from_airbase(airbases::MAYKOP, StartType::Runway)
    .key("red-cap-1") // spawn and destroy: always keyed
    .await?; // Group<Air>
cap.push_task(
    Task::cap(zones::SAM_RING)
        .altitude(ft(25_000))
        .speed(kts(420)),
)
.timeout(secs(5))
.await?;
cap.set_option(Roe::WeaponFree).await?;

let armor = ctx.group(groups::RED_ARMOR)?;
armor.destroy().key("armor-despawn").await?;
```

Options on every command:

| Option | Meaning |
| --- | --- |
| `.key(impl Into<String>)` | idempotency key, scope: the session (see the rules in SKILL.md) |
| `.timeout(Duration)` | fail with `sortie::timeout` if DCS has not answered; default 10 s, 30 s for `spawn_group` and `destroy` |
| `.on_pause(v1::PausePolicy::Fail)` | fail with `sortie::paused` instead of waiting while the simulation is paused (default: wait) |
| `.after(&other_cmd)` | run only after `other_cmd` (not yet awaited) completed |
| `.lane(v1::Lane::..)` | scheduling lane; the default fits the command |
| `.id()` | the command id, shown in errors |

| Command | Output | Notes |
| --- | --- | --- |
| `ctx.message(to).text(..).secs(u32).clear_view()` | `()` | `to`: `Coalition`, `&Group<K>`, `AnyGroup` (a menu press), `sortie::MessageTarget::All`; 10 s by default |
| `ctx.smoke(at, SmokeColor::Red)` | `()` | `at`: a `MapPoint` (on the ground) or a `Vec3` |
| `ctx.flag(flags::X).set(v)` / `.get()` | `()` / `i32` | `v`: `bool` or `i32` |
| `ctx.spawn_group(t)` | `Group<K>` | `t`: `templates::X` or `templates::X.named("Name")`; then `.from_airbase(airbases::Y, StartType::{Runway, Hot, Cold, Air})` or `.at(Vec3, heading_rad)`; without either, the template's own placement. Default name `<template name>#<session4>-<n>`, units `<group>-<k>`; `.named` fails with `sortie::name_conflict` if a live group has the name. The engine paces spawns (currently 2 groups or 24 units a second), so a big wave takes time. |
| `group.set_option(o)` | `()` | `o`: `Alarm::{Auto, Green, Red}` or `Roe::{WeaponFree, OpenFireWeaponFree, OpenFire, ReturnFire, WeaponHold}` |
| `group.destroy()` | `()` | despawn; no DEAD event follows (a `UnitGone` does) |
| `group.push_task(task)` / `group.set_task(task)` | `()` | `Group<Air>` and `Group<Helo>` only; push on the task stack or replace it |
| `ctx.menu(c).command(menus::X)` | `MenuCommand<S>` | see Menus |
| `ctx.end_session(outcome).text(..)` | `()` | ends the session (the turn); succeeds even if the session ends first |

Tasks (`Task` is in the prelude):

| Task | Defaults |
| --- | --- |
| `Task::cap(zones::X).altitude(Length).speed(Speed)` | 6000 m, 420 kt |
| `Task::orbit(MapPoint).altitude(..).speed(..).race_track(MapPoint)` | 6000 m, 350 kt, a circle without `race_track` |
| `Task::rtb(airbases::X)` | |
| `Task::raw(sortie::proto::LuaValue)` | a DCS task table, not checked |

Units and geometry: `m`, `km`, `ft`, `nm` → `Length`; `kts`, `mps`, `kmh` → `Speed`; `secs`,
`mins`, `hours` → `Duration`. `MapPoint { north, east }` (metres, map axes): `.alt(Length)` →
`Vec3`, `.distance(p)`, `.offset(north_m, east_m)`, `.bearing_to(p)`,
`.toward(bearing_rad, Length)`. `Vec3 { x: north, y: up, z: east }`: `.map_point()`. Zones:
`zones::X.center()`, `zones::X.contains(p)`.

Groups: `ctx.group(groups::X)?` returns the live group or `sortie::not_in_world` (no live unit:
late activation, destroyed) or `sortie::unknown_symbol` (the running mission lacks the symbol).
`Group<K>` is bound to one life of the group: a group respawned under the same name is a new
handle. Methods: `eid()`, `name()`, `alive_count()`, `is_alive()`, `any()` → `AnyGroup`.

## Keys in loops

```rust
for n in 1..=3 {
    ctx.timer(&format!("wave-{n}-delay"), mins(5)).await; // deadline stored in the engine
    let wave = ctx
        .spawn_group(templates::RED_CAP)
        .from_airbase(airbases::MAYKOP, StartType::Hot)
        .key(format!("wave-{n}")) // same n, same key: never spawned twice
        .await?;
    ctx.wait_until(|w| w.group(&wave).alive_count() == 0).await;
}
```

After a logic restart the loop runs again from `n = 1`: passed timers return at once, keyed
spawns return their stored groups, and the waits on dead groups pass. The script fast-forwards
to where it was.

## Once

`Campaign` here and below is a state document type:
`#[derive(Default, Serialize, Deserialize)] pub struct Campaign { pub sams_killed: u32 }`.

```rust
let campaign = ctx.state::<Campaign>("campaign").await?; // campaign scope: kept across turns

// Once per session: skipped when the function runs again after a crash.
ctx.once("briefing", async |ctx| {
    ctx.message(Coalition::Blue)
        .text("Destroy SAM North.")
        .secs(30)
        .await
})
.await?;

// Once per turn (= per mission build): also skipped after a mission restart.
ctx.wait_until(|w| w.group(&groups::SAM_NORTH).alive_count() == 0)
    .await;
ctx.once_per_turn("count-sam-kill", async |ctx| {
    campaign.update(|c| c.sams_killed += 1).await?;
    ctx.flag(flags::SAM_DOWN).set(true).await
})
.await?;
```

`ctx.once(name, f)` and `ctx.once_per_turn(name, f)` return `Ok(Some(value))` when `f` ran and
`Ok(None)` when it had run before. The mark is written after `f` succeeded: an `Err` from `f`
leaves it unmarked.

## Waiting

```rust
ctx.sleep(secs(30)).await; // model time; stops while paused; starts over after a crash
ctx.timer("cap-delay", mins(10)).await; // named: the deadline survives a logic restart

let blue_in_front = |w: &World| {
    w.units()
        .coalition(Coalition::Blue)
        .in_zone(zones::FRONT)
        .any()
};
ctx.wait_until(blue_in_front).await;

let red_left = ctx.world().group(&groups::RED_ARMOR).alive_count(); // a short borrow
ctx.report().set("red_armor_left", red_left);

// Any end: the logic, the time limit, a restart. Code after it runs only up to the next
// .await; then the session's tasks are aborted.
let end = ctx.until_session_end().await;
eprintln!("session ended: {:?} ({:?})", end.outcome, end.reason); // the logic's stderr
```

- `wait_until` checks its predicate now, after every tick (10 per second) and after every
  event. Keep the predicate cheap; see patterns.md for thousands of units.
- `sleep`, `timer`: model time; a pause stops them.
- `until_session_end()` → `SessionEnd { outcome: Option<Outcome>, text, reason }`, with
  `sortie::EndReason`: `Logic`, `TimeLimit`, `Restart`, `MissionUnloaded`, `EngineShutdown`,
  `Replaced`, `Disconnected`, `Unspecified`. When the session ends, pending commands fail with
  `sortie::session_ended` (not a logic error) and every task of the session is aborted.
- `ctx.model_time()`, `ctx.paused()`, `ctx.is_session_ended()`, `ctx.session_id()`,
  `ctx.turn()` (the build id of the running mission).

World queries (`ctx.world()` or the `wait_until` argument, a `&World`):

| Query | Methods |
| --- | --- |
| `w.units()` (live units) | filters `.coalition(c)`, `.air()`, `.planes()`, `.helos()`, `.ground()`, `.ships()`, `.statics()`, `.any_category()`, `.dead()`, `.including_dead()`, `.players()`, `.in_group(&g)`, `.of_type("F-16C_50")`, `.in_zone(zones::X)`, `.within(point, km(5))`; results `.count()`, `.any()`, `.first()`, `.iter()`, `.eids()`, `.nearest_to(point)`, `.inside(&zone)`, `.any_inside(zone)`, `.count_inside(zone)` |
| `w.players()`, `w.statics()` | the same query, preset |
| `w.group(&g)` | `g`: a `Group<K>`, `&groups::X`, a DCS group name; `.exists()`, `.alive_count()`, `.size()`, `.is_dead()`, `.center()`, `.any_inside(z)`, `.count_inside(z)`, `.all_inside(z)`, `.units()`, `.alive_units()`; a missing group counts as empty |
| `w.airbase_by_ident("MAYKOP")`, `w.airbase_by_name(..)`, `w.airbases()` | airbases with their current owner |

An entity (`&sortie::world::Entity`) has `name`, `type_name`, `coalition`, `category`,
`group_name`, `pos: Option<Vec3>`, `velocity`, `heading`, `player: Option<String>`, `life`,
`dead`.

## Events

```rust
let sam = ctx.group(groups::SAM_NORTH)?;
// Subscribe first, then act: a stream only sees events from the moment it was made.
let mut deaths = ctx.events().of::<v1::Dead>().in_group(&sam);
ctx.spawn(async move |ctx| {
    while let Some(dead) = deaths.next().await {
        let name = dead.initiator.map(|u| u.name).unwrap_or_default();
        ctx.message(Coalition::Blue)
            .text(format!("{name} destroyed"))
            .secs(10)
            .await?;
    }
    Ok(()) // `None`: the session ended
});
```

- `ctx.events()` → `Events`, `.next().await` → `Option<v1::Event>` (`ev.kind`,
  `ev.model_time`, `ev.seq`); `None` once the session ended.
- `.of::<E>()` keeps one type; filters `.in_group(&g)` (a `Group<K>` matches only its own life;
  a `GroupRef` or a name matches whatever group has it now), `.coalition(c)`, `.filter(|e| ..)`.
  They test the event's subject: the initiator, `unit` for `PlayerEnteredUnit`, `last_known`
  for `UnitGone`.
- Types: `Birth`, `Dead`, `UnitLost`, `Kill`, `Hit` (the initiator may be missing), `Shot`,
  `Takeoff`, `Land`, `BaseCaptured`, `Mark`, `PlayerEnteredUnit` (built from BIRTH and the
  player name: multiplayer clients do not get a reliable DCS event), `UnitGone` (left the world
  without DEAD: despawned, missing from a sample), `MenuPressed`, `PlayerConnected`,
  `PlayerDisconnected`, `PlayerChangedSlot`, `ChatMessage`, `SimulationPaused`,
  `SimulationResumed`, `Gap` (events were lost; the replica was resynced), `Unknown`.
- Each event is applied to the replica before your code sees it: in a `Dead` handler the unit
  already counts as dead.
- A stream sees events from the moment it was created. A stream that falls more than 1024
  events behind skips the oldest and logs a warning.

## Menus

```rust
let smoke = ctx
    .menu(Coalition::Blue)
    .per_group() // every blue player group gets the command; a press carries the group
    .command(menus::SMOKE_OBJECTIVE)
    .key("menu-smoke")
    .await?;
ctx.spawn(async move |ctx| {
    let mut presses = smoke.presses();
    while let Some(p) = presses.next().await {
        ctx.smoke(zones::OBJECTIVE.center(), SmokeColor::Red)
            .await?;
        ctx.message(p.group)
            .text("Red smoke on the objective")
            .secs(10)
            .await?;
    }
    Ok(())
});

let sitrep = ctx
    .menu(Coalition::Blue)
    .command(menus::SITREP)
    .key("menu-sitrep")
    .await?;
ctx.spawn(async move |ctx| {
    let mut presses = sitrep.presses();
    while presses.next().await.is_some() {
        let left = ctx.world().group(&groups::SAM_NORTH).alive_count();
        ctx.message(Coalition::Blue)
            .text(format!("SAM North: {left} alive"))
            .await?;
    }
    Ok(())
});
```

- `ctx.menu(coalition)` is one command for the whole coalition; a press is a
  `CoalitionPress { coalition, model_time }`. `.per_group()` installs it for every player group
  of the coalition (those present now and each new one at its first player BIRTH); a press is
  a `GroupPress { group: AnyGroup, coalition, model_time }`, and `ctx.message(p.group)` answers
  that group.
- Label and sub-menu path come from `scenario.py` (`sc.menu(ident, label, path=[..])`).
- Presses are queued from the moment the command is installed. Each press goes to one
  `Presses` stream. `menu.remove()` removes the command. Adding the same command twice changes
  nothing in DCS.

## Flags

```rust
let sam_down = ctx.flag(flags::SAM_DOWN);
sam_down.set(1).key("sam-down").await?; // bool or i32
let value: i32 = sam_down.get().await?; // reads DCS (a mission trigger may change it too)
```

## State and results

```rust
#[derive(Default, Serialize, Deserialize)]
struct Progress {
    waves_sent: u32,
}
// Campaign scope (default): kept across turns; the generator can read it.
let campaign = ctx.state::<Campaign>("campaign").await?;
// Turn scope: survives mission restarts of this build; session scope: this session only.
let turn = ctx
    .state_in::<Progress>(v1::StateScope::Turn, "progress")
    .await?;

turn.update(|p| p.waves_sent += 1).await?; // compare-and-swap, retried on conflict
let killed = campaign.get().sams_killed; // a local copy; do not hold `get()` across .await
ctx.report().set("sams_killed", killed); // MissionResults.report_json, read by `results`
```

| Scope | Lives | API |
| --- | --- | --- |
| `v1::StateScope::Session` | this session | `ctx.state_in::<T>(v1::StateScope::Session, name)` |
| `v1::StateScope::Turn` | this build (turn), across mission restarts | `ctx.state_in::<T>(v1::StateScope::Turn, name)` |
| `v1::StateScope::Campaign` | across turns of the scenario | `ctx.state::<T>(name)` |

`T: Serialize + DeserializeOwned + Default` (`#[derive(Default, Serialize, Deserialize)]`; the
crate needs `serde` with the `derive` feature). `StateDoc<T>`: `get()` (local copy),
`update(|t| ..)` (compare-and-swap; on a conflict it reloads and applies the closure again, up
to 8 times), `set(value)`, `reload()`, `revision()`. The engine stores the documents (SQLite)
and the generator reads them.

`ctx.report().set(key, value)` writes a key of the session's results (`scenario.py results`);
it does not wait, and a failure shows up as a logic error.

## Binary and environment

`sortie::main!(my_mission_logic::run)` makes the binary:

```text
my_mission_logic [--engine <url>] [--build-id <id>] [--logic <name>] [--dev]
  --engine     engine gRPC address (SORTIE_ENGINE, default http://127.0.0.1:50777)
  --build-id   build id announced to the engine (SORTIE_BUILD_ID, default <binary>@<version>)
  --logic      logic name (default: the binary name)
  --dev        spawns and destroys without .key() fail at the call site (SORTIE_DEV=1)
```

`SORTIE_TOKEN`: the API token (role `logic`; not needed with `sortied --dev`). `SORTIE_LOG`:
the log filter (default `info`; logs go to stderr). `--standby` (hot reload) is not available
yet. The binary reconnects with a backoff of 1 to 30 s. Exit codes: 0 after the engine shut it
down, 2 for bad arguments, a refused token or a rejected attach, 1 if the runtime could not
start.
