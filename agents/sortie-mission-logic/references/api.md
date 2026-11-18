# Sortie SDK API reference (for agents)

Everything below is in `sortie::prelude::*` unless a path is given. Event and state enums live
in `sortie::proto::v1` (`use sortie::proto::v1;`). The examples use the `frontline` scenario
(module `frontline`, marker `Frontline`).

## Commands

Every write returns a `Cmd<..>`. Nothing happens until it is awaited; `.await` gives
`sortie::Result<Output>`.

<!-- snippet: skills-logic-commands -->
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
| `Task::raw(sortie::proto::LuaValue)` | a DCS task table; the engine refuses Lua code in it (`Script`, `ScriptFile`, a `condition` string, a `file` string: `sortie::rejected`, capability `dev_eval`) and NaN or infinities (`sortie::invalid`) |

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

<!-- snippet: skills-logic-keys-loop -->
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

<!-- snippet: skills-logic-once -->
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

<!-- snippet: skills-logic-waits -->
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

<!-- snippet: skills-logic-events -->
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
  events behind skips the oldest and logs a warning. Holes in `ev.seq` are not losses (a
  filtered stream sees part of the journal); only `Gap` reports a loss.
- Which DCS event it was (engine API revision 1): `ev.dcs_event()` (`v1::DcsEvent`, the
  `world.event` entry by name; `Unspecified` for engine and hook events, and for an entry the
  enum lacks, which `ev.dcs_name` then names). `Dead` comes from DEAD, CRASH and PILOT_DEAD,
  `Takeoff` from TAKEOFF and RUNWAY_TAKEOFF, `Land` from LAND and RUNWAY_TOUCH: pick with
  `.from_dcs(v1::DcsEvent::Crash)` (repeatable: any of) and read the entry with
  `.next_with_dcs_event()` → `Option<(E, v1::DcsEvent)>`. Against a 0.1.0 engine
  `.from_dcs` matches nothing. From API revision 1 (`#[sortie::mission(api_rev = 1)]`) a
  pilot's death no longer kills the unit in the world; at revision 0 (the macro's default) the
  `Dead` of PILOT_DEAD reads `dead = true` as in 0.1.0 and the logic's world holds the unit
  dead for good. A player leaving a unit clears `EntitySnap.player` (every revision).
- Weapons and ejected crew are no world entities but have Eids (engine API revision 1, every
  revision of the logic): `Shot`, `Hit` and `Kill` of one weapon carry the same `weapon.eid`
  (0: no weapon object, only `weapon_type`, or a unit in a collision); pilots and seats in the
  `ctx` of `S_EVENT_EJECTION` / `S_EVENT_LANDING_AFTER_EJECTION` are snaps with Eids (seat and
  pilot differ). `Obj::weapon(&w)` / `Obj::snap(&s)` (`Option<Obj>`, `None` for Eid 0) select
  one for `ctx.destroy(..)` while DCS still has it (`caps::DESTROY_WEAPON`); afterwards, or 30 s
  after a weapon's first hit, `Error::Gone`. No `UnitGone` follows.
- Every other DCS event is `v1::Unknown` with `ctx: Option<v1::EventContext>`: `initiator`,
  `target` (snaps with Eids; Eid 0 for an object the engine cannot name), `weapon`, `place`
  (`airbase`, also for a ship's deck, or `entity`), `weapon_type`, `text` (or the comment),
  `pos`, `coalition`,
  `group_eid`, `idx`, `extra` (every other key as data, two levels), `extra_objects` (objects
  under other keys), `foreign` (another script raised it through `world.onEvent`: data only).
  `.in_group` / `.coalition` test `ctx.initiator`. `fields` is the raw table in the 0.1.0
  shape, the 0.1.0 keys only: read new keys from `ctx`.

<!-- snippet: skills-logic-events-dcs -->
```rust
// Wrecks only: DCS also sends the pilot's death as a Dead while the aircraft still flies.
let mut wrecks = ctx
    .events()
    .of::<v1::Dead>()
    .from_dcs(v1::DcsEvent::Dead)
    .from_dcs(v1::DcsEvent::Crash);
let mut starts = ctx
    .events()
    .of::<v1::Unknown>()
    .from_dcs(v1::DcsEvent::EngineStartup);
if let Some((dead, entry)) = wrecks.next_with_dcs_event().await {
    let name = dead.initiator.map(|u| u.name).unwrap_or_default();
    eprintln!("{name}: {entry:?}");
}
if let Some(start) = starts.next().await {
    let who = start.ctx.and_then(|c| c.initiator).map(|u| u.name);
    eprintln!("engines started: {who:?}");
}
```

- FakeDcs: `dcs.raise_event(FakeEvent::dcs("S_EVENT_ENGINE_STARTUP").initiator("Viper-1")
  .key("comment", ".."))`; another script's event: `FakeEvent::foreign(id)`.

## Menus

<!-- snippet: skills-logic-menus -->
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

## Menu trees

`ctx.menus()` builds F10 radio menu trees by name (API revision 1): commands and sub-menus for
everyone (the default), a coalition or one group. A tree is a resource of the session like a
drawing; a put with an existing name brings that tree to the new spec.

<!-- snippet: skills-api-menu-trees -->
```rust
#[sortie::mission(api_rev = 1, event_rev = 1)] // trees: API rev 1; their presses: event rev 1
pub async fn run(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // One group's tree: SUSPENDED until a player takes a slot of "Viper 1"; the engine installs
    // it for the group's id, removes it when the slot empties and follows a dynamic slot.
    let jtac = ctx
        .menus()
        .tree("jtac/viper-1")
        .to(Audience::group(GroupSel::name("Viper 1")))
        .under(["JTAC"])
        .command("check-in", "Check in")
        .submenu("Smoke", |s| {
            s.command_arg("smoke-red", "Red", "red")
                .command_arg("smoke-green", "Green", "green")
        })
        .await?;
    let mut presses = jtac.presses();
    while let Some(p) = presses.next().await {
        if p.item.starts_with("smoke-") {
            let color = if p.arg == "red" {
                SmokeColor::Red
            } else {
                SmokeColor::Green
            };
            ctx.smoke(zones::OBJECTIVE.center(), color).await?;
        }
        if let Some(group) = p.group {
            ctx.message(group).text("Copy").await?;
        }
    }
    ctx.menus().remove("jtac/viper-1").await?;
    ctx.end_session(Outcome::Draw).await
}
```

- `tree(name)` then `.to(..)`, `.under([..])`, `.command(id, label)`, `.command_arg(id, label,
  arg)`, `.manifest_command(id, menus::X)`, `.submenu(label, |s| ..)`, `.overflow(..)`,
  `.ttl(..)`; the result is `MenuTree { change, dcs_calls, resource, warnings }` with
  `.presses()` and `.is_suspended()`. A press is `TreePress { tree, item, arg, ident, group,
  coalition, model_time }`; `ctx.menus().presses(name)` reads presses of any tree.
- DCS shows 10 entries per level. A longer level of the tree is paged by default
  (`sortie::menu_tree::Overflow::Page`): 9 entries and a sub-menu `More...` with the rest,
  paged the same way. The page of the tree's root level reads `More... (<tree name>)`: that
  level is shared with other trees and DCS finds an entry by its label path, so each tree's
  page there needs its own label; pick tree names a player can read. Do not split a long list
  into sub-menus by hand for that, and do not look for the page in presses: a press of an
  entry on a page carries that entry's own `item` and `arg`. Pages cost DCS calls, not nodes.
  `.overflow(Overflow::Reject)` refuses such a level instead.
- `Error::Invalid`: two entries with one label on one level, an entry labelled like a page
  of its level (`More...`, on the root level also `More... (<tree name>)`) on a level that
  pages, more than 10 entries on a level with `Overflow::Reject`, more than 200 nodes, a
  command without an id.
- `change`: `Unchanged` (equal, no DCS call), `Updated` (only the changed entries and the ones
  after them on their level are sent again), `Replaced` (another audience or `under`).
- A group tree follows the slot: SUSPENDED while nobody occupies the group, installed by the
  engine when a player takes a slot, removed when it empties, installed again for a dynamic
  slot's new group id. A group given by Eid ends with `ResourceEnded{SOURCE_GONE}` with the
  group.
- `remove(name)`, `remove_prefix(p)`, `clear()`; menus go when the session ends. The 0.1.0
  `ctx.menu(..)` commands are registered too (`sortie:menu/..` in `ctx.world().resources()`).

## Flags

<!-- snippet: skills-logic-flags -->
```rust
let sam_down = ctx.flag(flags::SAM_DOWN);
sam_down.set(1).key("sam-down").await?; // bool or i32
let value: i32 = sam_down.get().await?; // reads DCS (a mission trigger may change it too)
```

## State and results

<!-- snippet: skills-logic-state -->
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

## DCS calls

`ctx.dcs()` reaches the DCS functions of the DCS API catalog (`catalog/dcs_api.toml` in the
SDK repository; the full list is `docs/dcs-catalog.md`) through generated, typed wrappers, for
what has no typed operation yet. The logic declares `#[sortie::mission(api_rev = 1)]`, else
every call is `Error::Revision` and nothing is sent.

<!-- snippet: skills-api-dcs -->
```rust
#[sortie::mission(api_rev = 1)] // ctx.dcs() is API revision 1
pub async fn run(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    let dcs = ctx.dcs();
    // Reads: no key (a key on a read is Error::Invalid).
    let surface = dcs.land().surface_type(zones::OBJECTIVE.center()).await?; // "ROAD"
    let sam: Vec<DcsObject> = dcs.group(groups::SAM_NORTH).units().await?;
    let fuel = dcs.unit(&sam[0]).fuel().await?;
    // Mutators: off until the engine owner enables them; always keyed.
    match dcs
        .airbase(airbases::MAYKOP)
        .set_coalition(Coalition::Blue)
        .key("maykop-blue")
        .await
    {
        Ok(()) | Err(Error::Rejected(_)) => {} // Rejected: switched off in engine.toml
        Err(e) => return Err(e),
    }
    let lase: Handle = dcs
        .spots()
        .create_laser(
            &sam[0],
            None,
            zones::OBJECTIVE.center().alt(m(0)),
            Some(1688.0),
        )
        .key("lase-1")
        .await?;
    dcs.spot(lase).destroy().key("lase-1-off").await?;
    let _ = (surface, fuel);
    ctx.end_session(Outcome::Draw).await
}
```

- Families: `land()`, `atmosphere()`, `timer()`, `coord()`, `coalition()`, `world()`, `env()`,
  `effects()`, `sounds()`, `spots()`, `net()` and `server()` (hooks); methods of an object:
  `unit(u)`, `group(g)`, `static_object(o)`, `airbase(a)`, `warehouse(w)`, `controller(c)`,
  `spot(handle)`, `object(o)`. A target is a scenario symbol, an `Eid`, a `Group`, a world
  entity, a `sortie::sel` selector or a `DcsObject` an earlier call returned. Inherited methods
  (`point()`, `coalition()`) are in every class's family; a method runs through the class of
  its object, as `unit:getCoalition()` does (the catalog path `CoalitionObject.getCoalition`
  is the entry's name: DCS has no `CoalitionObject` table).
- Each catalog entry is a capability `call:<env>:<path>` (state, signature, live check):
  `ctx.capability(caps::call::mission::LAND_GET_HEIGHT)` finds one ("Capabilities" below).
- Hooks-environment calls (`net()`, `server()`) need a running mission session in this release
  (wave 1), like mission calls: without one they are `sortie::unavailable`.
- Results are typed: enum numbers as their key (`"ROAD"`), coalitions as `Coalition`, points as
  `Vec3` / `MapPoint`, objects as `DcsObject { class, eid, name, type_name, .. }`, spots as
  `Handle`, tables as `sortie::dcs::Value`. A number DCS returns as a string (ids in 2.9.31)
  is still an `f64` where the catalog says `number` (`static_object(o).id()`).
- Point arguments follow the catalog entry: `vec2` is a `MapPoint` (`land().height(p)`), `vec3`
  a `Vec3` in DCS axes, x north, y up, z east (`land().profile(a, b)`: DCS reads x and z).
- A warehouse item (`item_count`, `add_item`, `remove_item`, `set_item`) is its name
  (`"weapons.missiles.AIM_9"`) or its wsType, four whole numbers: `[4, 4, 7, 22]` or
  `(4, 4, 7, 22)` (`sortie::dcs::WarehouseItem`); MOOSE's `ENUMS.Storage` uses both forms.
  `inventory(Some(..))` takes a name only. FakeDcs keeps a wsType's stock apart from its name's.
- Reads (`read`, `heavy`) take no key: a key on a read is `Error::Invalid`. Mutators (`mutate`)
  are off until the engine owner enables them in `<Saved Games>/Sortie/engine.toml` (`[calls]
  enable = [..]`) or runs `sortied --dev`; until then `Error::Rejected` naming the capability
  `call:mission:<path>`. In `enable` and `disable` a method is named by its catalog entry, the
  class that defines it (`Object.destroy`, not `Unit.destroy`): the entry is one for every
  class, and an inherited name stops `sortied` at start. A mutator repeats its effect when run
  again: key it. A repeat with the same key does not reach DCS and returns the stored result,
  objects and handles included (a replayed `create_laser(..).key("lase-1")` gives the same
  `Handle`); only a result above 64 KiB is stored without its values, and its repeat is
  `Error::ResultNotKept`.
- The engine checks every call against the catalog: wrong arguments are `Error::Invalid` with
  the entry's signature, an argument out of the entry's range (explosion power above 5000) too.
  A sound file is a path inside the `.miz` (relative, no `..`).
- A big table comes back whole or as `Value::Cut(n)` (its key count) with `truncated` on the
  `Return`: deeper than `.max_depth(n)` (6 by default), after 50 000 table entries of one call,
  or when the answer of the whole command, every call of a batch together, would pass 150 000
  values (objects count with their description). A wrapper that expects a list then fails
  with `Error::Invalid`; split a big read into several commands.
- An object, id or name argument must be of the class the entry declares, also when it is a
  `Handle` or the result of an earlier call of a batch: a spot is no `object:Object`, a
  unit's id no group id. The wrong class is `Error::Invalid`, and DCS is not called with it.
- The world replica does not learn what a mutator changed until the next sweep (airbase owner,
  destroyed objects). Prefer the typed operation where one exists: such wrappers are
  `#[deprecated]` and name it (`ctx.smoke(..)` for `effects().smoke(..)`).
- The untyped call by path is behind the cargo feature `dcs-raw`:
  `ctx.dcs().call("land.getHeight").arg(sortie::dcs::arg::vec2(p))`, and `ctx.dcs().batch()`.

## Objects

Static objects, cargo and FARPs, destroying any object, removing junk: typed operations of API
revision 1 (the engine keeps the name policy and the spawn gate and tells the world at once).

<!-- snippet: skills-api-objects -->
```rust
#[sortie::mission(api_rev = 1)] // spawn_static, destroy, remove_junk are API revision 1
pub async fn run(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    let p = zones::OBJECTIVE.center();
    // The engine fills the DCS category and shape from its type table.
    let depot = ctx
        .spawn_static(".Ammunition depot", "RUSSIA")
        .named("Depot")
        .at(p, 0.0)
        .on_conflict(OnConflict::Keep) // a rerun gets the live depot back
        .key("depot")
        .await?;
    // A working FARP is an airbase: `heliport` is required for heliport types.
    let farp = ctx
        .spawn_static("FARP", "USA")
        .named("FARP Alpha")
        .at(p.toward(0.0, m(3_000)), 0.0)
        .heliport(Heliport::new().callsign("Dallas"))
        .key("farp-alpha")
        .await?;
    // Destroy any unit, static or cargo; a unit with a player needs .allow_player().
    match ctx
        .destroy(Obj::static_object("Depot"))
        .key("depot-gone")
        .await
    {
        Ok(()) | Err(Error::Gone(_)) => {}
        Err(e) => return Err(e),
    }
    let junk = ctx
        .remove_junk([Volume::sphere_on_ground(p, 1_000.0)])
        .await?;
    ctx.report().set("farp", farp.airbase.is_some());
    ctx.report().set("junk", junk.total);
    let _ = depot.kept;
    ctx.end_session(Outcome::Draw).await
}
```

- `ctx.spawn_static(type, country)` -> `SpawnedStatic { eid, name, airbase, kept }`; place it with
  `.at(MapPoint, heading)` or `.on_unit(ship, forward_m, right_m, angle)`. The static is in
  `ctx.world()` when the command returns and a `Birth` with `synthesised = true` announces it
  (DCS raises none for statics). Key it: in dev mode it is refused without a key.
- The name is the identity (units and statics share it). Default `OnConflict::Fail` ->
  `Error::NameConflict`; `Keep` answers a live static of the same type (`kept = true`, a FARP
  with its `airbase`); `Replace` destroys a live static (its `UnitGone` says `Replaced`) and
  spawns. If DCS did not remove the old static (reported for FARPs, not measured), `Replace`
  is `Error::Lua` ("DCS did not remove ..") and nothing is added.
- The engine's type table (from pydcs) only supplies the default category and shape;
  `.category(..)` / `.shape(..)` win over it. Whether DCS knows the type is decided by DCS
  (`getDescByName`): an unknown type is `Error::Invalid`. A type the table lacks (a mod) needs
  both `.category(..)` and `.shape(..)` (unit categories need no shape); with one missing it is
  `Error::Invalid` naming the missing field. Heliport
  types need `.heliport(..)`; `.cargo(mass, liftable)` needs a cargo type.
- `ctx.destroy(obj)` takes an `Eid`, a world entity or `Obj::{unit, static_object, weapon,
  snap}` (a weapon in flight or an ejected pilot by its event Eid). A unit a player occupies is
  `Error::Rejected` unless `.allow_player()` (also on `group.destroy()` with API revision 1).
  Scenery is `Error::Unavailable`. The world gets `UnitGone` with `Despawned` (not for weapons
  and pilots: no entities); a FARP's airbase ends with its static. Removing a FARP is not
  promised: DCS is reported not to remove one with `destroy()` (not measured). The Lua layer
  looks again one DCS frame after it destroyed a static or cargo; when DCS still has it,
  `ctx.destroy` is `Error::Lua` ("DCS did not remove .."), no `UnitGone` comes and the static
  and its airbase stay alive in the world. FakeDcs: `dcs.survives_destroy(name, true)`.
- `ctx.remove_junk(volumes)` (1 to 16) -> `JunkRemoved { total, per_volume }`. DCS volume types
  and map circles only; a polygon is `Error::Invalid`, never widened. Safe to repeat.
- `ctx.search_objects(volume)` (API revision 2: `#[sortie::mission(api_rev = 2)]`) -> `Found {
  objects, truncated }`: what `world.searchObjects` finds in one volume (DCS volume types and
  map circles, as `remove_junk`). `.categories([ObjectCategory::Unit, Static, Scenery, ..])`
  (the default: units, statics, scenery), `.limit(n)` (default 500, at most 2000; more in the
  volume sets `truncated`). Each `FoundObject { category, eid, name, type_name, point, life,
  life_max, coalition, dcs_id }` is what DCS answered then; `eid` is set for a unit, static or
  cargo the world holds (read the rest from `ctx.world()`); scenery has only `dcs_id` (its map
  id). A read: it changes nothing. Types in `sortie::cmd`.

Many spawns at once (a campaign restore) are one `ctx.spawn_batch()` command with one key:

<!-- snippet: skills-api-batch -->
```rust
// A campaign restore: one command, one key, checked as a whole before anything is sent;
// the spawn gate spreads the items, the result comes when the last one settled.
let mut batch = ctx.spawn_batch();
let cap = batch.add_group(
    ctx.spawn_group(templates::RED_CAP.named("Red CAP 1"))
        .from_airbase(airbases::MAYKOP, StartType::Runway),
);
for (name, p) in &depots {
    batch = batch.static_object(
        ctx.spawn_static(".Ammunition depot", "RUSSIA")
            .named(name.as_str())
            .at(*p, 0.0),
    );
}
let out = batch.key("restore-turn-7").await?;
let cap: Group<Air> = out.get(&cap)?; // typed per item; an item's own error otherwise
for item in out.items.iter().filter(|i| !i.is_ok()) {
    ctx.report().set(item.name.as_str(), item.skipped); // nothing is rolled back
}
```

- Items are `ctx.spawn_group(..)` / `ctx.spawn_static(..)` builders, not awaited:
  `.group(..)` / `.static_object(..)` chain, `.add_group(..)` / `.add_static(..)` return a
  `BatchSlot<T>` for `out.get(&slot)` -> `Group<K>` or `SpawnedStatic`. 1 to 512 items; their
  own `.key`/`.timeout`/`.after` do not apply.
- The engine checks the whole batch first (each item as its own command, no two items with one
  name): one malformed item (unknown template, bad name) fails the command and nothing spawns.
  A name a live group or static (or a spawn in flight) already holds is that item's own
  `Error::NameConflict`; the others run (with `.stop_on_error()` the items after it are
  skipped). Then the spawn gate (2 groups and 24 units per second by default, a static is one
  of each) spreads the items, sharing the gate in turn with other spawns; each is applied to
  the world as it answers. The default deadline adds the gate's time.
- Nothing is rolled back: the command is OK when it ran; `SpawnedBatch::items` holds each
  item's `eid`, `error()`, `skipped`, `settled`. `.stop_on_error()` sends one item at a time and
  skips the rest after the first failure (`Error::Rejected`).
- Lost with the mission link: `Error::Unavailable`. Retry with the same key: until the world
  snapshot after the reconnect it answers `Unavailable` naming the unknown items; after it the
  retry resumes the batch (items found by name are `settled`, the rest spawn, none twice).
- Capability `caps::SPAWN_BATCH`. An engine owner who switched off `spawn_static` or
  `spawn_group` also refuses a batch that holds such an item (`Error::Rejected`, and
  `Error::capability()` names the item's operation).

## Map drawings

`ctx.map()` draws on the F10 map: `panel` (a mark with a text box), `line`, `path` (2..=33
points), `polygon` (3..=64, 4 = the DCS quad), `circle`, `rect`, `arrow`, `text`. Every drawing
is a named resource of the session (API revision 1); a put with an existing name brings that
drawing to the new spec.

<!-- snippet: skills-api-map -->
```rust
#[sortie::mission(api_rev = 1, event_rev = 1)] // drawings: API rev 1; ResourceEnded: event rev 1
pub async fn run(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    let obj = zones::OBJECTIVE.center();
    // A put by name: the same spec again sends nothing to DCS.
    let d = ctx
        .map()
        .circle("obj/ring", obj, 4_000.0)
        .color(Rgba::RED)
        .line_style(LineStyle::Dashed)
        .to(Coalition::Blue)
        .await?;
    assert!(matches!(d.change, Change::Created | Change::Unchanged));
    ctx.map()
        .panel("obj/label", obj, "Objective")
        .to(Coalition::Blue)
        .await?;
    // Ends that were not this logic's own remove.
    let mut ends = ctx.events().of::<v1::ResourceEnded>();
    if let Some(e) = ends.next().await {
        ctx.report().set("ended", e.name);
    }
    ctx.map().remove_prefix("obj/").await?;
    ctx.end_session(Outcome::Draw).await
}
```

- `Drawn.change`: `Created`; `Unchanged` (equal spec, no DCS call); `Updated` (colour, fill,
  circle radius, text and font size of a text, first or last point changed in place, or only
  the ttl); `Replaced` (anything else: new DCS marks). Commands on one name run in order; no
  key is needed (a key is ignored).
- `.color(Rgba)`, `.fill(Rgba)`, `.line_style(LineStyle)`, `.to(..)` (repeatable, at most 8;
  none = everyone; shapes reach everyone or a coalition, a panel also one group),
  `.editable()`, `.announce(text)`, `.font_size(n)`, `.ttl(Duration)`.
- `remove(name)` (`Change::Absent` when there is none), `remove_prefix(p)`, `clear()`: own
  drawings only unless `.any_owner()`.
- `ResourceEnded` (needs `event_rev = 1`): `EXPIRED` (ttl), `PLAYER` (a player deleted an
  editable mark), `LOST` (missing after a reconnect), `REMOVED_BY_OTHER`. The session's end
  removes everything without an event. `ctx.world().resources()` lists what exists.
- Mark ids from 7 100 000 up are Sortie's: the raw `ctx.dcs().markup()` wrappers are
  `#[deprecated]` and refuse such ids.

## Capabilities

One table says what the engine, its Lua layer and the running DCS build can do. Names are the
constants of `sortie::caps`: typed operations (`caps::SPAWN_STATIC`, `caps::SPAWN_BATCH`,
`caps::DRAW`, `caps::MENU_TREE`, `caps::REMOVE_JUNK`, `caps::DESTROY_WEAPON`, `caps::DCS_CALL`,
...) and one per DCS API catalog entry
(`caps::call::mission::LAND_GET_HEIGHT` = `"call:mission:land.getHeight"`).

<!-- snippet: skills-api-caps -->
```rust
#[sortie::mission(api_rev = 1, event_rev = 1, requires = [caps::SPAWN_STATIC])]
pub async fn run(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // Refused at Attach when spawn_static is not AVAILABLE: no code below runs then.
    let t = ctx.capabilities(); // Start's table, kept current by CapabilityChanged
    let report = ctx.report();
    if !t.is_available(caps::REMOVE_JUNK) {
        report.set("junk", t.note(caps::REMOVE_JUNK)); // why not
    }
    if !t.is_available(caps::call::mission::LAND_GET_SURFACE_TYPE) {
        report.set("surface", "MISSING_IN_DCS, DISABLED or QUARANTINED");
    }
    ctx.end_session(Outcome::Draw).await
}
```

- States: `AVAILABLE`; `NOT_IMPLEMENTED` (declared only, e.g. `caps::DESTROY_SCENERY`, or the
  Lua layer is older than the engine); `MISSING_IN_DCS` (the running DCS build lacks the
  function); `DISABLED` (the engine owner switched it off, mutators of the catalog by default,
  `dev_eval` without `--dev-eval`); `QUARANTINED` (DCS crashed while that kind of command ran,
  or the bridge disabled it after 3 faults). `note` says why. `live` is always `UNVERIFIED`.
  A typed operation's quarantine ends when DCS runs another build; the build is one per DCS
  process (the hooks link names it, else the mission link), and a link that reports `unknown`
  names none, so it neither lifts a quarantine nor keeps one from being recorded.
- `requires = [..]` goes into `Attach.required_capabilities`; a missing one refuses the attach
  (`LogicHandle::rejected()` in FakeDcs, `AttachRejected.missing_capabilities`).
- `CapabilityChanged` updates `ctx.capabilities()` only for `event_rev = 1`.
- A refusal names its capability: `e.capability()` (`Some("call:mission:land.getHeight")` for
  a quarantined catalog function). A DCS crash inside a catalog call quarantines that one
  function, named by the bridge's crash journal (without a journal entry: every function of the
  batch in flight); `dcs_call`, `dcs_batch`, other functions and typed operations stay
  `AVAILABLE`. It survives DCS and engine restarts and a new DCS build (then its `note` says
  "recorded on DCS <old build>, carried over to the running build <new build>"): the engine
  owner lifts it (`sortie caps --release call:mission:land.getHeight`, or every carried-over
  one with `sortie caps --release-carried-over`), or, for a documented read without a
  receiver, a passed probation call at the next mission start. A function gets one probation:
  if it faults again after a pass, it stays quarantined until the owner releases it. No
  probation runs while the owner has `dcs_call` switched off or the function is missing in
  the running DCS build (the `note` says "probation waits: ..."). A function the bridge
  refuses only because three calls of it took longer than a second (its `note` says "Slow,
  not a crash") is `QUARANTINED` until DCS restarts and comes back by itself: nothing is
  stored and nothing needs a release. An owner who sets
  `engine.toml [quarantine] clear_calls_on_new_build = true` lets a new DCS build lift it too.
  Code that must keep running
  checks `t.is_available(caps::call::mission::LAND_GET_HEIGHT)` (the function it needs) and
  falls back to a typed operation or another function.
- A typed operation that was running when DCS crashed (`message`, `spawn_static`, `draw`,
  `menu_tree`, ...) is quarantined on its own until the owner releases it or DCS runs another
  build; it refuses commands of API revision 1 only (a 0.1.0 logic is not affected). While
  `menu_tree` or `draw` is quarantined, every put of that kind fails with
  `Error::Quarantined`, also one that needs no DCS call, and the engine moves no tree with its
  slot and removes no expired drawing.
- A Lua layer older than the engine (no `sortie install` after an update) shows every wave-1
  operation as `NOT_IMPLEMENTED`; `requires = [..]` then refuses the attach.
- FakeDcs: `dcs.remove_dcs_function("world.removeJunk")` makes the fake DCS build lack it; the
  table shows `MISSING_IN_DCS` from the next mission load (`dcs.restart().await`).

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
