# Patterns for a long multiplayer mission (for agents)

A server mission runs for hours, holds thousands of units, and players come and go. The
patterns below keep it cheap and replay-safe. They use the `frontline` scenario.

## Shape: one task per concern

<!-- snippet: skills-pattern-layout -->
```rust
use sortie::prelude::*;
use sortie::proto::v1;
use sortie::types::Eid;

use frontline::{Frontline, airbases, groups, menus, templates, zones};

#[sortie::mission]
pub async fn run(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // One task per concern; each lives until the session ends.
    ctx.spawn(support_menu);
    ctx.spawn(players);
    ctx.spawn(red_waves);
    ctx.spawn(front_control);
    ctx.spawn(idle_end);

    // The turn ends when the objective falls; otherwise the manifest's time limit ends it.
    ctx.wait_until(|w| w.group(&groups::SAM_NORTH).alive_count() == 0)
        .await;
    ctx.end_session(Outcome::BlueWin)
        .text("SAM North destroyed")
        .await
}
```

- Each concern (menus, players, AI waves, zone control, the idle timeout) is a plain
  `async fn name(ctx: Ctx<M>) -> sortie::Result<()>` started with `ctx.spawn(name)`. An error
  in one is recorded and ends only that task.
- The mission function itself waits for the end condition and calls `end_session`. Without
  one, the time limit of the scenario (`time_limit=` in `scenario.py`) ends the turn.
- Every task is a replayable script: after a logic restart all of them start again from the
  top, so each step is keyed, `once`d, timed by a named timer or a wait on world state.

## Players joining and leaving

<!-- snippet: skills-pattern-players -->
```rust
/// Players joining and leaving: one loop over the raw event stream.
pub async fn players(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    let mut events = ctx.events();
    while let Some(ev) = events.next().await {
        match ev.kind {
            Some(v1::event::Kind::PlayerEnteredUnit(e)) => {
                let Some(unit) = e.unit else { continue };
                let group = AnyGroup {
                    eid: unit.group_eid.and_then(Eid::from_u64),
                    name: unit.group_name,
                };
                let text = format!("Welcome, {}. F10 > Support has smoke.", e.player);
                match ctx.message(group).text(text).secs(15).await {
                    // Gone: the player left the slot before the message went out.
                    Ok(()) | Err(sortie::Error::Gone(_)) => {}
                    Err(err) => return Err(err),
                }
            }
            Some(v1::event::Kind::PlayerDisconnected(e)) => {
                eprintln!("{} left", e.name); // the logic's stderr
            }
            _ => {}
        }
    }
    Ok(())
}
```

- `PlayerEnteredUnit` comes for every player who takes a slot, also on a respawn into a new
  aircraft. `PlayerConnected` and `PlayerDisconnected` (player id, name, `ucid`) and
  `PlayerChangedSlot` (player id, name, coalition, slot) come from the server.
- Who is flying now is world state: `w.players()` (units with a player), `.coalition(..)`,
  `entity.player`. After a logic restart the event history is gone; the world is not.
- Per-group menus (`.per_group()`) reach groups that spawn later on their own: no code per
  join.
- Event-driven messages need no key: a replay does not see old events, so it cannot repeat
  them.

## AI waves

<!-- snippet: skills-pattern-waves -->
```rust
/// AI pressure: paced by named timers, keyed per wave, capped, scaled to the players online.
pub async fn red_waves(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    let mut alive: Vec<Group<Air>> = Vec::new();
    for n in 1u32.. {
        ctx.timer(&format!("wave-{n}"), mins(15)).await;
        alive.retain(Group::is_alive);
        let blue = ctx.world().players().coalition(Coalition::Blue).count();
        if blue == 0 || alive.len() >= 4 {
            continue; // nobody to fight, or enough red in the air already
        }
        let cap = ctx
            .spawn_group(templates::RED_CAP)
            .from_airbase(airbases::MAYKOP, StartType::Hot)
            .key(format!("wave-{n}"))
            .await?;
        cap.push_task(Task::cap(zones::FRONT))
            .key(format!("wave-{n}-task"))
            .await?;
        alive.push(cap);
    }
    Ok(())
}
```

- A named timer per wave paces the loop and survives a logic restart; a key per wave makes the
  spawn and its task happen once.
- The decision (players online, red already in the air) reads the world as it is now. A replay
  decides again only for waves that never spawned.
- Cap the number of live AI groups. The engine paces spawns (currently 2 groups or 24 units a
  second) and a spawn waits at most 30 s by default: a wave of many groups is a loop of
  awaited spawns, not hundreds of commands at once.
- A spawned group's name is `<template name>#<session4>-<n>`; find spawned groups in queries by
  their `Group` handle, not by name.

## Thousands of units

- `wait_until` runs its predicate after every tick (10 a second) and every event. A predicate
  over a group (`w.group(&g).alive_count()`) or a zone (`.any_inside(zone)`, indexed by a
  spatial grid) is cheap. A predicate that scans every unit is not: poll it every few seconds
  with `ctx.sleep` instead.

<!-- snippet: skills-pattern-poll -->
```rust
/// A periodic check instead of `wait_until`: cheap with thousands of units.
pub async fn front_control(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    loop {
        ctx.sleep(secs(30)).await;
        let (red, blue) = {
            let w = ctx.world();
            let red = w
                .units()
                .coalition(Coalition::Red)
                .ground()
                .count_inside(zones::FRONT);
            let blue = w
                .units()
                .coalition(Coalition::Blue)
                .ground()
                .count_inside(zones::FRONT);
            (red, blue)
        }; // the borrow of the world ends here, before the next .await
        if red == 0 && blue > 0 {
            ctx.message(sortie::MessageTarget::All)
                .text("Blue holds the front")
                .key("front-taken")
                .await?;
            return Ok(());
        }
    }
}
```

- Symbols are for what the logic names. Ten scenario groups with idents and hundreds of
  anonymous ones (from templates or a `@sc.pydcs` hook) is normal: query those by zone, type
  (`.of_type(..)`), coalition, or DCS name (`.in_group("Name")`).
- An event stream that falls more than 1024 events behind drops the oldest. Keep each event
  loop short; hand slow work (spawns, many messages) to another task. Several typed streams
  (`.of::<v1::Dead>()`, `.of::<v1::Kill>()`) are cheaper to keep up than one loop that awaits
  inside.
- Read the world in a block and drop the borrow before the next `.await`.

## Idle server

<!-- snippet: skills-pattern-idle -->
```rust
/// Ends the turn after 30 minutes of model time without a player in a slot.
pub async fn idle_end(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    let mut idle_minutes = 0;
    loop {
        ctx.sleep(mins(1)).await;
        let online = ctx.world().players().count();
        idle_minutes = if online == 0 { idle_minutes + 1 } else { 0 };
        if idle_minutes == 30 {
            return ctx
                .end_session(Outcome::Aborted)
                .text("No players for 30 minutes")
                .await;
        }
    }
}
```

Model time stops while the simulation is paused; a server that pauses without players never
reaches the 30 minutes, which is what you want.

## Scores and other campaign data

<!-- snippet: skills-pattern-scores -->
```rust
/// Campaign-wide scores, kept by the engine.
#[derive(Default, Serialize, Deserialize)]
pub struct Scores {
    pub kills: std::collections::BTreeMap<String, u32>,
}

pub async fn scores(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    let scores = ctx.state::<Scores>("scores").await?;
    let mut kills = ctx.events().of::<v1::Kill>().coalition(Coalition::Blue);
    while let Some(kill) = kills.next().await {
        let Some(player) = kill.initiator.and_then(|u| u.player) else {
            continue; // an AI kill
        };
        scores
            .update(|s| *s.kills.entry(player.clone()).or_default() += 1)
            .await?;
    }
    Ok(())
}
```

- Campaign documents live in the engine and survive restarts of the logic, the mission and the
  engine; the generator reads them for the next turn.
- Every `update` is a round trip to the engine. One per kill is fine; for high-rate data
  (hits, shots) count locally and write once a minute.
- A logic restart does not replay old events, so an event-driven counter never counts twice.
  A counter driven by a replayed step (a wave, an objective) goes inside `ctx.once_per_turn`.

## Instead of → do

| Instead of | Do |
| --- | --- |
| a spawn or destroy without a key | `.key("step-name")` on every spawn and destroy |
| a key built from model time, a random number or an Eid | a key built from the step: a literal or a deterministic counter |
| the same key with a changing text | a key per distinct command, or no key for event-driven messages |
| `ctx.sleep` for a delay that must survive a restart | `ctx.timer("name", dur)` |
| waiting for a `Dead` event | a `wait_until` on `w.group(&g).alive_count() == 0` |
| `campaign.update(..)` in the open | inside `ctx.once_per_turn(..)` |
| holding `ctx.world()` or `doc.get()` across `.await` | a block that ends before the `.await` |
| `std::thread::sleep`, blocking I/O, `std::thread::spawn` | `ctx.sleep`, `ctx.spawn` |
| hundreds of spawns at once | a loop of awaited spawns with a cap |
| a `wait_until` predicate scanning every unit | a group or zone predicate, or a poll with `ctx.sleep` |
| `ctx.group(groups::X)?` on a group that may be dead | match `Error::NotInWorld`, or check `alive_count()` first |
| `.unwrap()` on command results | `?`, or a `match` on the errors the step expects (`Gone`, `SessionEnded`) |
| editing `scenario.lock.json` | `python scenario.py lock` |
