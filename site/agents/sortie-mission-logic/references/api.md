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
| `ctx.spawn_group(t)` | `Group<K>` | `t`: `templates::X` or `templates::X.named("Name")`; then `.from_airbase(airbases::Y, StartType::{Runway, Hot, Cold, Air})`, `.at(Vec3, heading_rad)` (unit 1 at the point, the other units moved with their template offsets, not turned, and **every unit facing `heading_rad`**) or `.at_formation(Vec3, heading_rad)` (API revision 3: the formation turned as one body about unit 1 by `heading_rad` minus the template's reference heading, `Template.heading_rad` else unit 1's, each unit facing its template heading plus that turn, so the differences between the units stay); both give the group one route point at the point (the template's route is not carried over); without either, the template's own placement. Switching a keyed `.at` to `.at_formation` is another command: give it a new key (the same key is `Error::Invalid`). Default name `<template name>#<session4>-<n>`, units `<group>-<k>`; `.named` fails with `sortie::name_conflict` if a live group has the name. The engine paces spawns (currently 2 groups or 24 units a second), so a big wave takes time. |
| `group.set_option(o)` | `()` | `o`: `Alarm::{Auto, Green, Red}` or `Roe::{WeaponFree, OpenFireWeaponFree, OpenFire, ReturnFire, WeaponHold}` |
| `group.destroy()` | `()` | despawn; no DEAD event follows (a `UnitGone` does) |
| `group.push_task(task)` / `group.set_task(task)` | `()` | every group; push on the task stack or replace it. `Task::cap` / `Task::orbit` / `Task::rtb` are for `Group<Air>` and `Group<Helo>`, `Task::ground_route` for `Group<Ground>` (a task of the wrong kind does not compile, except RTB, which the engine refuses: `sortie::invalid`); `Task::raw` for any group, and the only task of a naval group |
| `ctx.menu(c).command(menus::X)` | `MenuCommand<S>` | see Menus |
| `ctx.end_session(outcome).text(..)` | `()` | ends the session (the turn); succeeds even if the session ends first |

Tasks (`Task` is in the prelude):

| Task | Defaults |
| --- | --- |
| `Task::cap(zones::X).altitude(Length).speed(Speed)` | 6000 m, 420 kt |
| `Task::orbit(MapPoint).altitude(..).speed(..).race_track(MapPoint)` | 6000 m, 350 kt, a circle without `race_track` |
| `Task::rtb(airbases::X)` | |
| `Task::ground_route([RoutePoint::on_road(p), RoutePoint::new(p, GroundFormation::Vee).speed(..)]).speed(Speed)` | API revision 3, `Group<Ground>` only; 20 km/h; 1 to 256 points |
| `Task::raw(sortie::proto::LuaValue)` | a DCS task table, for any group; the engine refuses Lua code in it (`Script`, `ScriptFile`, a `condition` string, a `file` string: `sortie::rejected`, capability `dev_eval`) and NaN or infinities (`sortie::invalid`) |

### A ground group's route

A ground group drives a typed route, `Task::ground_route` (API revision 3: the mission declares
`#[sortie::mission(api_rev = 3)]`; `RoutePoint` and `GroundFormation` are in the prelude):

```rust
let armor = ctx.group(groups::RED_ARMOR)?; // Group<Ground>
let objective = zones::OBJECTIVE.center();
let route = Task::ground_route([
    // along the roads to a point 3 km south of the objective
    RoutePoint::on_road(objective.offset(-3_000.0, 0.0)),
    // the last leg straight across the fields, in a vee, at 15 km/h
    RoutePoint::new(objective, GroundFormation::Vee).speed(kmh(15)),
])
.speed(kmh(40)); // for the points that set no speed (default 20 km/h)
armor.set_task(route).key("armor-to-objective").await?;
```

- The engine builds DCS's `Mission` task in the shape the Mission Editor writes for vehicles,
  with a first point where the group's first live unit stands in the frame DCS takes the task
  (the formation and speed of the first route point). Do not add a start point yourself.
- A point's formation and speed are those of the leg that ends at it (the editor's model: a
  leg keeps to the roads when both its ends are "On Road", and is timed by the speed of the
  point it ends at).
- Formations: `RoutePoint::on_road` (along the roads), `off_road` (straight, in column),
  `on_railroads` (trains), `RoutePoint::new(p, GroundFormation::{Rank, Cone, Vee, Diamond,
  EchelonLeft, EchelonRight})` (straight, in that formation).
- Speed: the point's, else the route's, else 20 km/h.
- Points are `MapPoint`s: `zones::X.center()`, a unit's `pos.map_point()`, a point of
  `ctx.dcs().land().path_on_roads(..)` ("DCS calls", "A road path") or of its summary
  `ctx.dcs().land().road_path(..)` ("The summary of a road path"). 1 to 256 points; a longer
  route is `sortie::invalid`. Between two "On Road" points the Mission Editor lays the road
  path itself (that DCS drives it so is `AI-L4`), so a route needs only the places where it
  turns or stops: of a road path, its end (`path.last()`) or a few points where it turns. Never
  map every point of a road path into the route: a long path can have more points than the
  limit, and each would be a leg of its own. The whole path is for measuring the way.
- `Group<Ground>` only: for an aircraft, a helicopter or a ship it does not compile (through
  `Task::from(route)` the engine refuses it, `sortie::invalid`). A logic below revision 3 gets
  `sortie::revision`, an older engine `sortie::engine_too_old`; nothing is sent.
- How DCS drives it (which road, reaching the nearest road, a point in water or off the map) is
  not measured live yet: live check `AI-L4` of the wide probe.

What the typed route lacks (a stop, tasks at waypoints, a `Custom` formation, a ship's route)
goes in a raw DCS task table, `Task::raw(..)`, which any group takes:

```rust
use sortie::proto::LuaTable;
use sortie::proto::LuaValue;

// DCS's "Hold" task stops the group where it stands; the SDK has no typed stop.
let stop = LuaTable::new()
    .with("id", "Hold")
    .with("params", LuaTable::new());
let armor = ctx.group(groups::RED_ARMOR)?;
armor.push_task(Task::raw(LuaValue::table(stop))).await?;
```

- A waypoint of a raw `Mission` task is `{type = "Turning Point", action = "On Road" | "Off
  Road", x, y, speed}`: `x` north and `y` east in map metres (`MapPoint.north` / `.east`;
  `Vec3.x` / `.z`), `speed` in m/s; start it where the group stands, as the editor does. How
  DCS drives a raw route is live check `AI-L3`.
- The engine checks a raw table for safety only; a malformed task can fail in DCS
  (`sortie::lua_error`) or be ignored by DCS without an error.
- SDK 0.3.0 and older had `push_task` / `set_task` on aircraft groups only, and no typed route.

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
  `ctx.turn()` (the build id of the running mission). Model time is seconds since the mission
  started (`timer.getTime`), standing still while paused, never the time of day (a mission that
  starts at 09:00 reads about 60 a minute in, not 32 460); ticks, `sleep`, `timer` and every
  event's `model_time` use it. The time of day: `ctx.dcs().timer().abs_time()`.

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
  `SimulationResumed`, `Gap` (events were lost; the replica was resynced), `Unknown`; of event
  revision 1 `ResourceEnded`, `CapabilityChanged`; of event revision 2 `AirbaseOwnerChanged`
  ("Airbases" below). A kind above the logic's event revision never arrives
  (`#[sortie::mission(event_rev = N)]`, capped by the engine's: `ctx.event_rev()`); `.of::<E>()`
  logs a warning then.
- A `Mark` (F10 map mark) carries model time like every event since 0.4.0; DCS's own `time`
  of it (the time of day, per the Hoggit wiki; live check EVT-L18) is `Mark.dcs_time` (API
  revision 3). With a Lua layer older than 0.4.0 (no `sortie install`) `model_time` of a mark
  is still the time of day and `dcs_time` is empty; the engine's clock is not affected.
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

## Documents for the web API

A logic publishes named JSON documents; the engine stores them and serves them read-only over
its HTTP API (`GET /api/v1/docs/<logic>/<name>`, a list at `/api/v1/docs`, a change stream at
`.../events`) for a site's live map or campaign page. API revision 3:
`#[sortie::mission(api_rev = 3)]`.

```rust
#[derive(Serialize)]
struct Front {
    sam_alive: usize,
    objective: [f64; 2], // lat, lon: the web API itself serves DCS metres only
}
// ctx.publish needs #[sortie::mission(api_rev = 3)]; ctx.dcs() is API revision 1.
let center = zones::OBJECTIVE.center().alt(m(0.0));
let (lat, lon, _alt) = ctx.dcs().coord().lo_to_ll(center).await?;
let sam_alive = ctx.world().group(&groups::SAM_NORTH).alive_count();
let front = Front {
    sam_alive,
    objective: [lat, lon],
};
let p = ctx.publish("front", &front).await?; // served at GET /api/v1/docs/<logic>/front
let _ = p.changed; // false: the engine had this JSON already (same `p.rev`)
let _removed = ctx.unpublish("old-front").await?; // false when there was none
```

- `ctx.publish(name, &value)` (`value: impl Serialize`, written with serde_json when called) ->
  `sortie::publish::Published { rev, updated_unix_ms, changed }`, once the engine stored it.
  The same JSON again changes nothing (`changed == false`, same `rev`): publishing on a timer
  is cheap. `ctx.unpublish(name)` -> `bool`, whether a document was removed.
- A document belongs to the logic (its `Attach.logic` name, the binary's name), not to the
  session: it outlives the session, the logic process and the engine (the state database).
  Publish when the data changes; there is nothing to restore at start.
- Rules, else `Error::Invalid` and nothing is stored: a name is 1 to 64 bytes of `a-z`, `0-9`,
  `_`, `-`; a document is at most 256 KiB of JSON; a logic has at most 64 documents; the
  logic's own name is 1 to 64 bytes of `A-Z`, `a-z`, `0-9`, `.`, `_`, `-`.
- `rev` grows with every publish of any document of the engine and is never reused while the
  engine keeps its state database; an engine with its state in memory counts from 1 again
  after a restart. The HTTP `ETag` (and the stream's event id) is `<epoch>-<rev>`, where the
  epoch names the database: one ETag is one content, also across restarts.
- The web API serves points in DCS metres only: publish latitude and longitude yourself
  (`ctx.dcs().coord().lo_to_ll(point)`).
- A logic below revision 3 gets `Error::Revision`, an older engine `Error::EngineTooOld`;
  nothing is sent.
- HTTP callers never reach the logic or DCS; a site cannot change a document.

## Actions for the web API

A logic declares named actions; a site or a tool with an `operator` (or `admin`) token calls one
with `POST /api/v1/actions/<logic>/<name>` and a JSON body; the engine hands the request to the
logic, and the logic's answer is the HTTP response. The logic decides what happens in the game
(validate the body like any outside input). API revision 3: `#[sortie::mission(api_rev = 3)]`.

```rust
#[derive(Deserialize)]
struct Smoke {
    color: String, // "red" | "green"
}
#[derive(Serialize)]
struct Smoked {
    smoked: bool,
}
// ctx.api() needs #[sortie::mission(api_rev = 3)]. A site calls
// POST /api/v1/actions/<logic>/smoke_objective {"color": "red"} with an operator token.
let mut smoke = ctx.api().action("smoke_objective")?; // declared now, withdrawn on drop
ctx.spawn(async move |ctx| {
    while let Some(req) = smoke.next().await {
        let color = match req.json::<Smoke>() {
            Ok(s) if s.color == "red" => SmokeColor::Red,
            Ok(s) if s.color == "green" => SmokeColor::Green,
            Ok(_) => {
                req.fail(422, "color is red or green")?; // 4xx only
                continue;
            }
            Err(e) => {
                req.fail(400, e)?;
                continue;
            }
        };
        ctx.smoke(zones::OBJECTIVE.center(), color).await?;
        req.reply(&Smoked { smoked: true })?; // HTTP 200 {"smoked":true}; once per request
    }
    Ok(())
});
```

- `ctx.api().action(name)` -> `sortie::Result<sortie::api::ActionRequests>`: declares the
  action at once; `next().await` -> `Option<ActionRequest>`, `None` when the session ended.
  Dropping the stream or the end of the session withdraws the action; the mission function
  declares it again in the next session and after a restart of the logic process (until then
  callers get 503 `unavailable`).
- `ActionRequest`: `json::<T>()` (`Error::Invalid` whose text says what did not fit),
  `body()` (the JSON bytes; an empty body is `null`), `token()` (`Option<&str>`, the caller's
  token name), `role()` (`"operator"` or `"admin"`), `name()`.
- Answer once: `reply(&value)` -> 200 with that JSON (at most 1 MiB); `fail(status, message)`
  with a status 400..=499 -> `{"error": {"code": "refused", "message": ..}}`. Both consume the
  request and return `sortie::Result<()>` (`Error::Invalid` for a value that is not JSON, too
  large, or a status outside 4xx: the caller then gets 500). A request dropped unanswered
  (end of the loop, `?`, the session's end) is answered 500 `unanswered` by the SDK.
- The engine waits 10 s (`sortie::api::ACTION_TIMEOUT`), then answers 504 itself. Answer
  first and do long work afterwards (`ctx.spawn`), reporting progress in a published document.
  A 504 does not say whether the action ran: let the caller send an order id and key the
  commands with it (`.key(..)`), so a retried call does not run twice.
- Read the stream promptly: a request that waited 10 s in the queue is never handed to the
  logic (its caller got 504); the SDK answers it `unanswered` when the next one arrives or on
  `next()`.
- Rules, else `Error::Invalid` and nothing is declared: a name is 1 to 64 bytes of `a-z`,
  `0-9`, `_`, `-`; at most 64 actions; one stream per action at a time; the logic's own name is
  1 to 64 bytes of `A-Z`, `a-z`, `0-9`, `.`, `_`, `-`. Below revision 3 `Error::Revision`, an
  older engine `Error::EngineTooOld`.
- The engine also refuses before the logic sees anything: another role (403), a body that is
  not JSON (400) or above `[http] max_body_bytes` (413), more than `[http] actions_per_s` calls
  of one token (429; counted per token, not per token name). `GET /api/v1/actions` lists what
  attached logics declare.

## DCS calls

`ctx.dcs()` reaches the DCS functions of the DCS API catalog (`catalog/dcs_api.toml` in the
SDK repository; the full list is `docs/dcs-catalog.md`) through generated, typed wrappers, for
what has no typed operation yet. The logic declares `#[sortie::mission(api_rev = 1)]`, else
every call is `Error::Revision` and nothing is sent.

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
        .sounds()
        .out_sound("l10n/DEFAULT/alarm.ogg")
        .key("alarm-1")
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
- A warehouse item (`item_count`; the deprecated `add_item`, `remove_item`, `set_item`: change
  stock with `ctx.warehouse_op()`, "Warehouses") is its name
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
  A sound file is a path inside the `.miz` (relative, no `..`). Guard `span_300km`: the two ends
  of `land().profile(..)`, `land().path_on_roads(..)` and `land().road_path(..)` lie at most
  300 km apart on the map, else `Error::Invalid` and nothing reaches DCS.
- Limits per writer (the logic's token name, else its name), refused at submit with
  `Error::Rejected` "rate limit: ..." and nothing sent. Counts: read 200/s, mutate 50/s, heavy
  2/s, admin 1/s (a batch takes all its calls; a 256-read batch leaves under 0.3 s of debt).
  DCS time (`cost_us`, the time a command took inside DCS's frame), charged when the result
  comes back, a late result after `Error::Timeout` too: reads and every batch, a batch with a
  mutate included, pay from `dcs_time.read` (20 ms a second, 20 ms at most; a read or batch
  that cost more refuses the writer's single reads for at most 1 s, its next batch until the
  whole debt is paid back); heavy calls pay from their own
  `dcs_time.heavy` (20 ms a second, 200 ms at most, the whole debt kept: after a heavy call of
  C ms the next heavy call waits about (C - 200) / 20 s, 102 s after a 2241 ms road path, and
  reads such as `timer().time()` pass at once). One heavy call of a writer at a time: the next
  one sent while the previous is not back is `Error::Rejected` (`DcsTime`, `dcs_time.heavy`, no
  `retry_after()`); send heavy calls one after the other, not in `join!`. The debt belongs to
  the writer: restarting the logic does not clear it. A single mutate call pays no DCS time.
- A big table comes back whole or as `Value::Cut(n)` (its key count) with `truncated` on the
  `Return`: deeper than `.max_depth(n)` (6 by default), after 50 000 table entries of one call,
  or when the answer of the whole command, every call of a batch together, would pass 150 000
  values (objects count with their description). A wrapper that expects a list then fails
  with `Error::Invalid`; split a big read into several commands.
- An object, id or name argument must be of the class the entry declares, also when it is a
  `Handle` or the result of an earlier call of a batch: a spot is no `object:Object`, a
  unit's id no group id. The wrong class is `Error::Invalid`, and DCS is not called with it.
- The world replica does not learn what a mutator changed until the next sweep (airbase owner:
  the next resync snapshot, then an `AirbaseOwnerChanged{Discovered}`; destroyed objects). Prefer the typed operation where one exists: such wrappers are
  `#[deprecated]` and name it (`ctx.smoke(..)` for `effects().smoke(..)`).
- The untyped call by path is behind the cargo feature `dcs-raw`:
  `ctx.dcs().call("land.getHeight").arg(sortie::dcs::arg::vec2(p))`, and `ctx.dcs().batch()`.
- An engine owner's entry (`engine.toml` `[[calls.call]]`) is called by path. Its `enum:<table>`
  type names the whole path of a published DCS table: 2 to 6 Lua names from an enum root (`AI`,
  `Airbase`, `Controller`, `Group`, `Object`, `Spot`, `StaticObject`, `Unit`, `Weapon`,
  `coalition`, `country`, `land`, `radio`, `trigger`, `world`), so
  `enum:AI.Option.Ground.val.ALARM_STATE` loads (0.3.0 refused 5 names); a data root (`env`,
  `net`, `DCS`, `timer`) or 7 names stops `sortied` at start. Pass a key,
  `arg::enum_key("AI.Option.Ground.val.ALARM_STATE", "RED")` (the layer reads the running DCS's
  number), or (0.4.0) one of the table's numbers, `arg::number(2.0)`, which the layer checks is a
  value of the table (`Error::Invalid` otherwise; a layer before 0.4.0 refuses numbers there).
  Alarm states and ROE have the typed `group.set_option(..)`, which needs no owner entry.

### A road path

`ctx.dcs().land().path_on_roads(..)` asks DCS for a path along roads between two map points
(`land.findPathOnRoads`). It is API revision 3: `#[sortie::mission(api_rev = 3)]`, else
`Error::Revision`; an engine of an older revision is `Error::EngineTooOld`.

```rust
#[sortie::mission(api_rev = 3)] // the road path is API revision 3
pub async fn convoy_distance(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    let armor = ctx.group(groups::RED_ARMOR)?;
    let objective = zones::OBJECTIVE.center();
    let Some(here) = ctx.world().group(&armor).center() else {
        return ctx.end_session(Outcome::Draw).await;
    };
    // Road type, then x (north) and y (east) of the start and of the end.
    let points = ctx
        .dcs()
        .land()
        .path_on_roads(
            "roads",
            here.north,
            here.east,
            objective.north,
            objective.east,
        )
        .await?; // Option<Vec<MapPoint>>, start to end; None: DCS answered nil
    // The length along the points; None (null in the report) when DCS found no path.
    let km: Option<f64> =
        points.map(|p| p.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>() / 1000.0);
    ctx.report().set("road_km_to_objective", km);
    ctx.end_session(Outcome::Draw).await
}
```

- Arguments as DCS takes them: the road type, then x (north) and y (east) of the start and of
  the end, in map metres (`MapPoint.north` / `.east`).
- Road type `"roads"`. For railroads the Hoggit wiki documents `"railroads"` and reports that
  DCS reads `"rails"`; which one works is not measured yet.
- The answer is `Option<Vec<MapPoint>>`, start to end. `None` is DCS's nil (the Mission
  Editor reads nil as "no path found"). What DCS returns for a point no road reaches (nil, an
  empty list, a path to the nearest road), the spacing of the points and how long a long path
  takes are not measured yet: live check `ENV-LC7` of the wide probe.
- Class `heavy`: DCS searches inside its frame, so a long path on a big map stops the server
  for its whole run (it cannot be made asynchronous). Heavy calls run one at a time on BULK, at
  most 2 per second per writer, never in a batch, take no key, and pay their DCS time from the
  writer's `dcs_time.heavy`: after a long one only the next heavy call waits. Three calls of
  one function slower than 1 s in 10 minutes and the bridge refuses it until DCS restarts
  (`Error::Quarantined`, "Slow, not a crash"). Ask once per route, not every tick.
- The ends lie at most 300 km apart (`span_300km`), else `Error::Invalid` before DCS.
- A convoy drives with a DCS `Mission` task ("A ground group's route"); DCS keeps `"On Road"`
  waypoints on the roads itself. Ask for the path when the logic needs the points: the
  distance, a stop on the way, whether a road connects two places at all.
- A long path is thousands of points. For its length and its ends, ask for the summary
  instead ("The summary of a road path").

### The summary of a road path

`ctx.dcs().land().road_path(road_type, start, end, max_points, keep)` asks DCS for the same
path, but the Sortie Lua layer summarises it inside DCS in the same frame and only the summary
travels: `Option<sortie::dcs::RoadPath>`. It is the layer's own function, catalog entry
`sortie.findPathOnRoads`: API revision 3, class `heavy` like the full path.

```rust
#[sortie::mission(api_rev = 3)] // the road path summary is API revision 3
pub async fn convoy_route(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    let armor = ctx.group(groups::RED_ARMOR)?;
    let objective = zones::OBJECTIVE.center();
    let Some(here) = ctx.world().group(&armor).center() else {
        return ctx.end_session(Outcome::Draw).await;
    };
    // Counted and measured inside DCS; at most 6 points travel: both ends, the sharpest turns.
    let summary = ctx
        .dcs()
        .land()
        .road_path("roads", here, objective, Some(6.0), None)
        .await?; // Option<RoadPath>; None: DCS found no path
    let Some(path) = summary else {
        ctx.report().set("road_km_to_objective", "no road");
        return ctx.end_session(Outcome::Draw).await;
    };
    ctx.report()
        .set("road_km_to_objective", path.length_m / 1000.0);
    ctx.report().set("road_points_in_dcs", path.count);
    // The route: the kept turns after the start, along the roads, if the road ends near.
    if path.points.len() > 1 && path.end_gap_m.is_some_and(|gap| gap < 500.0) {
        let legs = path.points[1..].iter().map(|&p| RoutePoint::on_road(p));
        armor
            .set_task(Task::ground_route(legs))
            .key("red-armor-road")
            .await?;
    }
    ctx.end_session(Outcome::Draw).await
}
```

- `count` (points of the DCS path), `length_m` (along them), `first` / `last` and
  `start_gap_m` / `end_gap_m` (metres from the first point to the start asked, from the last to
  the end asked: a road may stop short) are of the whole path.
- `points`: at most `max_points` of its points (2..=10 000, else `sortie::invalid` before
  DCS), both ends kept, start to end. `keep`: `None` or `Some("turns")` keeps the points where
  the path turns most (fewer when the rest lie on straight legs), `Some("even")` points spaced
  evenly along it; another word is `sortie::invalid`. `max_points: None` gives no points (the
  summary alone).
- `thinned`: `points` holds fewer points than the path (always so without `max_points`, except
  for an empty path: `count == 0` is never thinned).
- `None` is DCS's nil (no path); an empty DCS list is `count == 0` with no ends.
- The ends are two `MapPoint`s (the full path takes four numbers).
- A route from it: the kept points after the first as `RoutePoint::on_road(..)`, which DCS
  joins along the roads itself ("A ground group's route"); 256 route points at most.
- Needs the Lua layer of 0.4.0 (`sortie install`): an older layer has no `sortie.*` functions
  and answers MISSING (`Error::Unavailable`). That the summary matches the full path on a
  running DCS is live check `ENV-LC7`.

## Objects

Static objects, cargo and FARPs, destroying any object, removing junk: typed operations of API
revision 1 (the engine keeps the name policy and the spawn gate and tells the world at once).

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
- A FARP made at runtime: dynamic slots, a hot start and unlimited stock have no typed option.
  DCS documents no function or key for them; `.raw(..)` with MOOSE's `dynamicSpawn` /
  `allowHotStart` is an unmeasured candidate (live checks AWC-T29, AWC-T32), never a promise to
  a client. Forum reports say such a FARP starts with an empty warehouse: stock it with a keyed
  `ctx.warehouse_op().set(..)` (jet fuel as `Liquid::JetFuel`, helicopters by type name), and
  keep the key stable so a restarted logic does not set the stock back. What the server's slot
  screen holds: `ctx.dcs().server().farps_and_carriers()`, `.dynamic_spawn_settings(id, false)`,
  `.aircraft_in_warehouse(type, id, true)` (API revision 3; not what a client is shown).
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
- What DCS gives a script about the ground, for choosing a spawn site: no forest or tree data
  and no collision volume of rocks or map objects, in any documented function (a dedicated
  server too); Sortie has no "is this site clear" query. `land().surface_type(p)` knows only
  `LAND`, `SHALLOW_WATER`, `WATER`, `ROAD`, `RUNWAY` (a forest is `LAND`); `land.isVisible` and
  `land.getIP` see the terrain only; `ctx.search_objects` lists no trees (ED forum reports, not
  measured: OBJ-T32), so an empty result means no known object, not open ground. The
  undocumented `Disposition` table misses forests on Syria (VEAF, 2026) and is not used. Pick
  sites from what can be checked: slope and roughness from a grid of `land().height(..)` reads
  around each vehicle, `ctx.search_objects` radius checks, and zones of checked sites (and
  forbidden zones around forests) drawn in the scenario, tested in Rust with no DCS call, the
  choice kept in `ctx.state(..)` across a logic restart (sdk/docs/objects.md, "Что DCS знает о
  местности").

Many spawns at once (a campaign restore) are one `ctx.spawn_batch()` command with one key:

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

## Airbases

The owner, DCS's auto capture and the ATC's radio silence of 1 to 32 airbases in one command
(`cmd::MAX_AIRBASES`; a longer list goes in several commands, each its own frame): a typed
operation of API revision 3 (`#[sortie::mission(api_rev = 3)]`).

```rust
#[sortie::mission(api_rev = 3)] // ctx.airbase_control is API revision 3
pub async fn run(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // The front moved: both bases to blue, DCS's own capture off, in one frame.
    let out = ctx
        .airbase_control([airbases::MAYKOP, airbases::KRYMSK])
        .owner(Coalition::Blue)
        .auto_capture(false) // applied before the owner: DCS does not take the base back
        .key("front-turn-3")
        .await?;
    // The world has the new owner as soon as the command returned (nothing to wait for).
    let blue = ctx
        .world()
        .airbase_by_name("Krymsk")
        .is_some_and(|a| a.coalition == Some(Coalition::Blue));
    for base in &out.bases {
        if let Some(e) = base.error() {
            // This base only (Error::Gone: DCS has no such base now; Error::Lua: a setter
            // raised); the others ran.
            ctx.report().set(&base.name, e.code());
        }
    }
    // Refused before DCS: a base the world does not hold, one listed twice, nothing to set.
    match ctx
        .airbase_control([AirbaseSel::name("Nowhere")])
        .radio_silent(true)
        .await
    {
        Err(Error::Invalid(_)) => {}
        other => {
            other?;
        }
    }
    ctx.report().set("blue", blue);
    ctx.end_session(Outcome::Draw).await
}
```

- `ctx.airbase_control(bases)` takes anything `Into<AirbaseSel>`: `airbases::X`,
  `AirbaseSel::name("Kutaisi")`, the Eid of the airbase or of the FARP static or ship that
  carries it. Set at least one of `.owner(Coalition)`, `.auto_capture(bool)`,
  `.radio_silent(bool)`; the rest is left as it is. DCS applies the whole list in one frame,
  per base: read, auto capture, owner, radio, read again (auto capture first, so DCS does not
  hand the base back to the ground units near it). That is about 11 DCS calls a base inside
  one frame, hence at most 32 bases; what a base costs is not measured yet (AWC-T20).
- Result `AirbasesControlled { bases }`: one `AirbaseControlled { eid, name, before, after }`
  per base in the order given; `before` / `after` are `AirbaseState { owner, auto_capture,
  radio_silent }` as DCS read them (`None` where DCS did not answer). `base.error()` is that
  base's own error (`Error::Gone`: DCS has no such base now; `Error::Lua`: a setter raised,
  `after` says what DCS reads now); the other bases still ran and the command is OK.
  `out.ok_count()`.
- The new owner DCS read back is in `ctx.world()` when the command returns: nothing to wait
  for. A capture DCS raised before the command (still on its way when the result came) does
  not undo it: the engine orders the two by DCS's order of events. Keyed like any mutator (a repeat with the key returns the stored answer and does not
  reach DCS); no key is required in dev mode, but a hot reload runs it again without one.
- Refused before anything reaches DCS (`Error::Invalid` naming it): a base the world does not
  hold (an unknown name, a scenario airbase the mission lacks), one listed twice (also under
  two selectors), an empty or too long list, nothing to set. A base whose host was destroyed:
  `Error::Gone`. A logic of a lower revision: `Error::Revision`; an engine of API revision 2
  or less: `Error::EngineTooOld`.
- Capability `caps::AIRBASE_CONTROL` (on by default; an engine owner may switch it off:
  `Error::Rejected`). The raw wrappers `ctx.dcs().airbase(..).set_coalition(..)`,
  `.auto_capture(..)`, `.set_radio_silent_mode(..)` are `#[deprecated]` in favour of it (after
  them the world learns the owner only at the next resync snapshot or a state query of the
  base: nothing sweeps airbases); the reads are not.
- FakeDcs: `dcs.airbase_coalition(name)`, `dcs.airbase_auto_capture(name)`,
  `dcs.airbase_radio_silent(name)`.

### The owner event

`v1::AirbaseOwnerChanged` (event revision 2: `#[sortie::mission(api_rev = 3, event_rev = 2)]`;
`api_rev = 2, event_rev = 2` does not compile): exactly one per change of an airbase's owner in
the engine's world, whatever told the engine. It covers the captures DCS reports and the
changes the engine makes or reads; an owner changed elsewhere without an event (another
script's `setCoalition`) reaches the world only at the next resync snapshot or a state query,
so a logic that must see those polls `ctx.airbases_state(..)` (below).

```rust
#[sortie::mission(api_rev = 3, event_rev = 2)] // AirbaseOwnerChanged is event revision 2
pub async fn owners(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // One event per change of an airbase's owner, whatever told the engine.
    let mut owner_events = ctx.events().of::<v1::AirbaseOwnerChanged>();
    // Only DCS's own captures of Maykop (ground units took it).
    let mut maykop = ctx
        .events()
        .of::<v1::AirbaseOwnerChanged>()
        .airbase(airbases::MAYKOP)
        .cause(v1::OwnerChangeCause::Captured);
    while let Some(e) = owner_events.next().await {
        let name = e
            .airbase
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_default();
        // The world has the new owner already; previous() is Unspecified when none was known.
        let cause = match e.cause() {
            v1::OwnerChangeCause::Captured => "captured", // S_EVENT_BASE_CAPTURED
            v1::OwnerChangeCause::Control => "control",   // an AirbaseControl
            _ => "discovered",                            // a read of DCS found it
        };
        ctx.report().set(
            &name,
            format!("{:?} -> {:?} {cause}", e.previous(), e.owner()),
        );
        if e.owner() == v1::Coalition::Red {
            break;
        }
    }
    let _ = maykop.next().await;
    ctx.end_session(Outcome::Draw).await
}
```

- Fields: `airbase` (the snap after the change), `previous()` (`Unspecified`: the engine knew no
  owner), `owner()`, `cause()`: `Captured` (DCS raised `S_EVENT_BASE_CAPTURED`; right after its
  `BaseCaptured`, which every revision still gets), `Control` (an `airbase_control`; before the
  command's result), `Discovered` (a read of DCS found another owner: the resync snapshot after
  a link drop or lost events, a FARP spawn that kept the FARP).
- One event per change: a report of what the world holds already (DCS's capture of the owner an
  `airbase_control` just set, a repeated capture, a snapshot that confirms an event) is no
  second event; there and back is two. A capture DCS raised before an `airbase_control` or a
  state query read the base, which reaches the engine after their result, changes nothing (its
  `BaseCaptured` still comes; `ctx.world()` takes its owner until the next world update gives
  the engine's back). The session's first snapshot publishes nothing.
- Not seen at once: an owner DCS changed without an event (another script, the deprecated raw
  `set_coalition`) arrives only with the next resync snapshot or a state query of the base
  (below); one changed and changed back
  between two reads is never seen. Whether DCS raises `S_EVENT_BASE_CAPTURED` for
  `setCoalition` is not measured (AWC-T21).
- Filters: `.airbase(sel)`, `.cause(c)`, `.coalition(c)` (the new owner). The web API streams it
  as `airbase_owner_changed`.
- FakeDcs: `dcs.capture_airbase(name, coalition)` (DCS's capture, with its event),
  `dcs.set_airbase_coalition_silently(name, coalition)` (no event), `dcs.lose_events(1)`
  (the engine takes a resync snapshot: `Discovered`), `dcs.hold_events()` /
  `dcs.release_events()` (DCS's events wait in the bridge while command results go out).

### Parking, runways and the control state

Typed queries of API revision 3 (reads: nothing changes in DCS, no key): one base with
`ctx.airbase(sel)` and `.parking()`, `.runways()`, `.state()`; many with
`ctx.airbases_parking(..)`, `ctx.airbases_runways(..)`, `ctx.airbases_state(..)`.

```rust
#[sortie::mission(api_rev = 3)] // the airbase queries are API revision 3
pub async fn layout(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // The free spots of one base: only what DCS lists as free.
    let free = ctx
        .airbase(airbases::MAYKOP)
        .parking()
        .available_only()
        .await?;
    let helo = free
        .iter()
        .filter(|s| s.terminal == Some(TerminalType::HelicopterOnly))
        .count();
    // Its runways: course in radians as DCS gives it; heading_deg() reads the documentation's
    // way (-course, in degrees).
    let runways = ctx.airbase(airbases::MAYKOP).runways().await?;
    let heading = runways.first().and_then(|r| r.heading_deg());
    // Many bases in one query (DCS reads 32 bases a frame); each base its own result.
    let read = ctx
        .airbases_state([airbases::MAYKOP, airbases::KRYMSK])
        .await?;
    for base in &read.bases {
        match (base.value, base.error()) {
            (Some(s), _) => ctx.report().set(&base.name, format!("{:?}", s.owner)),
            (None, Some(e)) => ctx.report().set(&base.name, e.code()), // e.g. sortie::gone
            (None, None) => {}
        }
    }
    ctx.report().set("helo_spots", helo);
    ctx.report().set("heading", heading);
    ctx.end_session(Outcome::Draw).await
}
```

- `.parking()` → `Vec<ParkingSpot { term_index, terminal, point, taken, taken_by,
  taken_by_name, dist_to_runway, to_ac, term_index_0 }>` in DCS's order. `terminal` is
  `Option<TerminalType>`: `Runway` (16), `HelicopterOnly` (40), `Shelter` (68),
  `OpenOrShelter` (72), `SmallShelter` (100), `OpenAir` (104), `Unknown("unknown(<n>)")` for a
  number the SDK has no name for. `taken`: the spot is not among the free ones DCS lists
  (`getParking(true)`). `taken_by`: the world's Eid of an object DCS named on the spot (`TO_AC`,
  documented as a boolean, is in `to_ac`). `.available_only()`: the free spots only
  (`s.is_free()`).
- `.runways()` → `Vec<Runway { name, course, length, width, center }>`; `course` is radians as
  DCS gives it (the documentation: the negative of the true heading); `heading_deg()` converts
  that way (not measured). A FARP or a ship has none.
- `.state()` → `AirbaseState { owner, auto_capture, radio_silent }`. An owner DCS reads that the
  world held otherwise goes into `ctx.world()` at once and is one
  `AirbaseOwnerChanged{Discovered}` (unless DCS reported a capture after the read: the world
  keeps that newer owner): polling the state is how a logic finds an owner another script
  changed without an event.
- One base: its error is the query's (`Error::Gone`: DCS or the world has no such base, a stale
  Eid; `Error::Lua`: a DCS read raised). Many bases (parking 1..=16, runways and state
  1..=256, each once) → `AirbasesRead { bases: Vec<AirbaseRead { eid, name, value }> }`,
  `value` `None` with `base.error()` set for a base that failed (also `Error::Rejected`: its
  parking did not fit the answer; ask for it alone), `read.ok_count()`. The query fails as a
  whole (`Error::Invalid`) only for an empty or too long list, a base listed twice, an unknown
  scenario symbol or an Eid that is no airbase. A name the world does not know is still sent.
- The three are read in parts so a long list does not stall DCS: the state and the runways 32
  bases per frame, parking about 300 spots per frame, at most 8000 spots per answer.
- Capabilities `caps::AIRBASE_PARKING`, `caps::AIRBASE_RUNWAYS`, `caps::AIRBASE_STATE` (on by
  default, each with its own DCS function). A lower declared revision: `Error::Revision`; an
  engine of API revision 2 or less: `Error::EngineTooOld`.
- FakeDcs: every manifest airfield has four spots (1 open air, 2 open or shelter, 3 shelter, 4
  helicopters only) and runway "9" (heading 90), a FARP four helicopter spots and no runway;
  `dcs.take_parking_spot(base, n, Some("Unit"))` / `dcs.free_parking_spot(base, n)`.

## Warehouses

The stock of warehouses, a typed query of API revision 3 (a read: nothing changes in DCS, no
key): one with `ctx.warehouse(sel).stock()`, many with `ctx.warehouses_stock([..])`. A warehouse
is an airbase's (`airbases::X`, `WarehouseSel::airbase(..)`), a cargo static's
(`WarehouseSel::cargo(..)`) or DCS's by name (`WarehouseSel::named(..)`). Changing the stock is
the command `ctx.warehouse_op()` (below); the raw `add_item`, `remove_item`, `set_item` and the
liquid setters are deprecated.

```rust
#[sortie::mission(api_rev = 3)] // the stock query is API revision 3
pub async fn resupply_check(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // One warehouse whole: each class is Unlimited (DCS lists it empty) or Limited(name -> n).
    let stock = ctx.warehouse(airbases::MAYKOP).stock().await?;
    let limited = matches!(stock.weapons, Some(StockClass::Limited(_)));
    // Many warehouses, only one item and the fuel: getItemCount / getLiquidAmount, cheaper.
    let read = ctx
        .warehouses_stock([
            WarehouseSel::airbase(airbases::MAYKOP),
            WarehouseSel::airbase(airbases::KRYMSK),
        ])
        .items(["weapons.bombs.Mk_82"])
        .liquids()
        .await?;
    for w in &read.warehouses {
        let line = match (&w.value, w.error()) {
            (Some(s), _) => match s.count("weapons.bombs.Mk_82") {
                Some(Amount::Counted(n)) => format!("{n} Mk-82"),
                Some(Amount::Unlimited) => "unlimited".to_owned(), // a count above 2^29
                None => "no count".to_owned(),
            },
            (None, Some(e)) => e.code().to_owned(), // e.g. sortie::gone
            (None, None) => String::new(),
        };
        ctx.report().set(&w.name, line);
    }
    ctx.report().set("maykop_limited", limited);
    ctx.end_session(Outcome::Draw).await
}
```

- Without `.items(..)` and `.liquids()` the whole inventory is read (`Warehouse.getInventory`,
  one warehouse a frame, a big one copied 400 entries a frame): `WarehouseStock { aircraft, weapons, liquids }`, each class
  `StockClass::Unlimited` (DCS lists it as an empty table, which it is reported to do for an
  unlimited class; whether an empty limited class reads the same is not known) or
  `StockClass::Limited(BTreeMap<name as DCS gives it, count>)`; `class.get(name)` →
  `Option<Amount>`. `liquids` is `Liquids::Unlimited` or `Liquids::Amounts(BTreeMap<Liquid,
  Amount>)` (`JetFuel`, `AviationGasoline`, `Mw50`, `Diesel`, `Unknown(..)`; kilograms assumed,
  DCS documents no unit).
- `.items([..])` (names or wsTypes `[a, b, c, d]`, the `WarehouseItem` of the raw wrappers,
  1..=64, each once) counts only those (`getItemCount`); `.liquids()` reads the four liquids
  (`getLiquidAmount`); either replaces the whole inventory (cheaper: 64 DCS calls a frame, some
  4 to find each warehouse and 1 a read).
  `stock.count(item)` → `Option<Amount>`: `Amount::Counted(n)`, or `Amount::Unlimited` for a
  count above 2^29 (what DCS is reported to answer for an unlimited stock; not measured). This
  read alone cannot tell an unlimited class that DCS answers with a small number: read the
  whole inventory for the class.
- One warehouse: its error is the query's (`Error::Gone`: no such warehouse in DCS, its base or
  static dead in the world, a stale Eid; `Error::Lua`: a read raised). Many (1..=64, each once
  by its DCS name, whatever the selector) → `WarehousesRead { warehouses: Vec<WarehouseRead {
  warehouse, name, eid, value }> }`, `w.error()` per warehouse (also `Error::Rejected`: its
  inventory did not fit the answer of at most 20 000 entries), `read.ok_count()`. The query
  fails as a whole (`Error::Invalid`) for an empty or too long list, a warehouse or item listed
  twice, an empty item name, an unknown symbol, a cargo Eid that is no static.
- Capability `caps::WAREHOUSE_STOCK` (on by default). A lower declared revision:
  `Error::Revision`; an engine of API revision 2 or less: `Error::EngineTooOld`.
- FakeDcs: a warehouse the test did not set is unlimited in every class (an airfield without
  `munitions`); `dcs.set_warehouse_stock(name, FakeStock::unlimited().weapons([(item, n)]))`,
  `dcs.warehouse_stock(name)`, `dcs.warehouse_count(name, item)`; the raw `Warehouse.*` calls
  read and change the same stock (an unlimited class takes no change).

Changing stock: `ctx.warehouse_op()` (API revision 3), `.set(w, what, n)`, `.add(..)`,
`.remove(..)`, `.transfer(from, to, what, n)`, 1..=16 changes in one DCS frame, all or nothing.

```rust
#[sortie::mission(api_rev = 3)] // WarehouseOp is API revision 3
pub async fn convoy_arrived(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // One command, one DCS frame, all or nothing: move six bombs, add fuel, set a count.
    let out = ctx
        .warehouse_op()
        .transfer(
            airbases::KRYMSK,
            airbases::MAYKOP,
            "weapons.bombs.Mk_82",
            6.0,
        )
        .add(airbases::MAYKOP, Liquid::JetFuel, 5_000.0)
        .set(airbases::KRYMSK, [4, 4, 7, 22], 10.0) // an item by wsType
        .key("convoy-1-arrived") // every warehouse_op needs a key in dev, sets too
        .await?;
    if !out.applied {
        // Nothing changed: each change says why (NotEnough, Unlimited, Gone, ...).
        for c in out.failed() {
            ctx.report()
                .set(&c.warehouse.name, format!("{:?}: {}", c.outcome, c.message));
        }
    }
    ctx.end_session(Outcome::Draw).await
}
```

- `what` is a `StockItem`: an item (name, wsType `[a, b, c, d]`, `WarehouseItem`; whole counts up
  to 1 000 000) or a liquid (`Liquid::JetFuel`, ...; up to 100 000 000, kilograms assumed). `set`
  takes 0 too, the others more than 0. A warehouse may appear in many changes; a transfer is one
  step (removed there and added here in the same frame, or not at all).
- All or nothing: every count is read and every change checked first, in order, against what
  the changes before it leave (the warehouse exists in DCS, the stock is not unlimited, enough
  to remove or transfer); one failure and nothing changes. While they run, each step's count is
  read before it and after it: a change DCS raises on, whose count does not move as asked, or
  whose count an earlier change already moved, undoes the ones before it.
- `out: StockChanges { applied, undo_failed, changes }`; per change `StockChanged { what,
  outcome, message, warehouse: StockSide { name, eid, before, after, unlimited }, to:
  Option<StockSide> }`; `out.failed()` gives the changes that kept it from applying.
  `StockOutcome`: `Done`, `NotApplied` (another change failed), `Undone`, `UndoFailed` (the stock
  stays as `after` says), `Unlimited` (a count above 2^29: refused, never "done"), `NotEnough`
  (more than the warehouse holds: refused, never clamped), `Gone`, `DcsError`, `Unchanged` (DCS
  took the change and the count did not move), `Aliased` (an earlier change of the command moved
  this count under another form of the item: its name and its wsType are one stock).
- A key: every `warehouse_op` needs `.key(..)` in dev (and with `sortied --require-keys`;
  `Error::Invalid` otherwise), a list of sets too: a reload runs it again, and unkeyed an add,
  remove or transfer moves stock twice and a set puts a count back over what was used since. A
  keyed repeat returns the stored answer and leaves the stock alone, a refusal too: retry with a
  new key.
- The command fails as a whole (`Error::Invalid`, nothing sent) for no change or more than 16,
  a warehouse the world does not hold alive, a transfer to its own warehouse, an amount out of
  range or a fractional item count, `Liquid::Unknown`; `Error::Gone` for a dead base or a stale
  Eid. Do not name one item by its name and by its wsType in one command (checked as two
  counts: the later change is `Aliased` and the command is undone).
- Frame cost: all in one frame, about 4 DCS calls a set, add or remove, 8 a transfer, 4 a
  warehouse: 16 changes of one item at one warehouse some 0.5 ms, 16 transfers between 32
  warehouses some 2.6 ms (estimates); keep commands between many warehouses short.
- Capability `caps::WAREHOUSE_OP` (on by default); lane REALTIME. FakeDcs runs it on the same
  stock, as the layer checks it (an unlimited class: `Unlimited`; two forms of one item:
  `Aliased` and an undo; its setters never fail, so no `DcsError` there).

The names of items: `ctx.item_names()` (API revision 3; a read that takes no key: with
`.key(..)` it is `Error::Invalid`) asks the running DCS
(`Warehouse.getResourceMap()`, DCS 2.8.8 and later) what it calls each object it knows and the
wsType of each. It is how a logic checks DCS's item ids before it stocks a warehouse (a FARP's
weapons by name, its helicopters by type): look the campaign's list up with `.items([..])` and
send only `names` to `ctx.warehouse_op()`. The DCS API catalog has no `Warehouse.getResourceMap`
entry; do not ask an owner to add one. Do not hard-code DCS strings or wsTypes from elsewhere: the last level of many
wsTypes is assigned when DCS loads its database, so it differs between builds and module sets.

```rust
#[sortie::mission(api_rev = 3)] // the names query is API revision 3
pub async fn name_the_stock(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // What this DCS calls each item and its wsType: read once per DCS process, then from memory.
    let names = ctx.item_names().await?;
    // The campaign's own list, checked against DCS before any stock query uses it.
    for item in ["weapons.bombs.Mk_82", "weapons.missiles.AIM_120C"] {
        let ws = names.ws_type_of(item); // None: DCS has no such name (or it is no item type)
        ctx.report().set(item, format!("{ws:?}"));
    }
    // A wsType from a mission's munitions, back to the name the warehouses use.
    let back = ctx
        .item_names()
        .items([WarehouseItem::from([4, 4, 7, 22])])
        .await?;
    let first = back.names.first().map(|n| n.name.clone());
    ctx.report().set(
        "4.4.7.22",
        format!("{first:?} unknown {}", back.unknown.len()),
    );
    ctx.end_session(Outcome::Draw).await
}
```

- `ItemNames { names: Vec<ItemName { name, ws_type: Option<[u32; 4]> }>, unknown, total, skipped,
  dcs_version, cached, source }`, `names` sorted by name, each once; weapons are named as the
  warehouses name them (`weapons.missiles.AIM_120C`), aircraft and other units by type.
  `ws_type` is `None` where DCS gives `{0, 0, 0, 0}` (shells, statics, turrets: no item type).
  `get(name)`, `ws_type_of(name)`, `names_of(ws)` (usually one name), `knows(item)`;
  `ItemName::item()` gives the `WarehouseItem` the stock query and `warehouse_op` take.
- `.items([..])` (names or wsTypes, 1..=64, each once) answers only the matching entries (a wsType
  every name that has it) and lists the rest in `unknown`. Without it the answer is the whole
  map (thousands of names): read it once at mission start and keep it.
- The engine reads DCS's map once per DCS process and version (the copy takes some pumps, 400
  entries a pump; no other heavy call runs meanwhile) and answers every later query from memory
  (`cached = true`), a mission restarted in the same DCS too; nothing is written to disk or the
  engine's store. Capability `caps::ITEM_NAMES` (on by default;
  `MISSING_IN_DCS`, and `Error::Unavailable`, on a DCS without the function). The generator's
  `sc.airbase(munitions = ..)` still takes wsTypes: names need a running DCS.
- FakeDcs: `dcs.set_item_names([(name, [a, b, c, d]), ..])` is the map (empty until set; a
  mission reload keeps it); a wsType the map gives one name is that name's stock in the fake's
  warehouses.

## Map drawings

`ctx.map()` draws on the F10 map: `panel` (a mark with a text box), `line`, `path` (2..=33
points), `polygon` (3..=64, 4 = the DCS quad), `circle`, `rect`, `arrow`, `text`. Every drawing
is a named resource of the session (API revision 1); a put with an existing name brings that
drawing to the new spec.

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
