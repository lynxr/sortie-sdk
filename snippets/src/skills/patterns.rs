//! Examples of `sortie-mission-logic/references/patterns.md`: a long multiplayer mission.

use super::frontline;

// snippet:start skills-pattern-layout
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
// snippet:end skills-pattern-layout

/// Per-group F10 command.
pub async fn support_menu(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    let smoke = ctx
        .menu(Coalition::Blue)
        .per_group()
        .command(menus::SMOKE_OBJECTIVE)
        .key("menu-smoke")
        .await?;
    let mut presses = smoke.presses();
    while let Some(p) = presses.next().await {
        ctx.smoke(zones::OBJECTIVE.center(), SmokeColor::Red)
            .await?;
        ctx.message(p.group).text("Smoke is out").await?;
    }
    Ok(())
}

// snippet:start skills-pattern-players
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
// snippet:end skills-pattern-players

// snippet:start skills-pattern-waves
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
// snippet:end skills-pattern-waves

// snippet:start skills-pattern-poll
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
// snippet:end skills-pattern-poll

// snippet:start skills-pattern-idle
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
// snippet:end skills-pattern-idle

// snippet:start skills-pattern-scores
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
// snippet:end skills-pattern-scores
