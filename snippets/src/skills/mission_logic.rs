//! Examples of the `sortie-mission-logic` skill (SKILL.md and references/api.md).

use super::frontline;

// snippet:start skills-logic-skeleton
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
// snippet:end skills-logic-skeleton

use frontline::{airbases, flags, menus, templates};
use sortie::proto::v1;

/// Commands: build, set options, `.await` the result from DCS.
pub async fn commands(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    // snippet:start skills-logic-commands
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
    // snippet:end skills-logic-commands
    Ok(())
}

/// Keys that stay stable when the function runs again: derived from the step, not from time.
pub async fn keyed_waves(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    // snippet:start skills-logic-keys-loop
    for n in 1..=3 {
        ctx.timer(&format!("wave-{n}-delay"), mins(5)).await; // deadline stored in the engine
        let wave = ctx
            .spawn_group(templates::RED_CAP)
            .from_airbase(airbases::MAYKOP, StartType::Hot)
            .key(format!("wave-{n}")) // same n, same key: never spawned twice
            .await?;
        ctx.wait_until(|w| w.group(&wave).alive_count() == 0).await;
    }
    // snippet:end skills-logic-keys-loop
    Ok(())
}

/// Campaign state that a mission restart must not count twice.
#[derive(Default, Serialize, Deserialize)]
pub struct Campaign {
    pub sams_killed: u32,
}

/// `ctx.once` and `ctx.once_per_turn`.
pub async fn once(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    // snippet:start skills-logic-once
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
    // snippet:end skills-logic-once
    Ok(())
}

/// Waiting by model time and by world state.
pub async fn waits(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    // snippet:start skills-logic-waits
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
    // snippet:end skills-logic-waits
    Ok(())
}

/// Typed events, filtered, in a task of their own.
pub async fn events(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    // snippet:start skills-logic-events
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
    // snippet:end skills-logic-events
    Ok(())
}

/// F10 menus: per group (presses say who) and coalition-wide.
pub async fn menu(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    // snippet:start skills-logic-menus
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
    // snippet:end skills-logic-menus
    Ok(())
}

/// Flags: set and read.
pub async fn flag(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    // snippet:start skills-logic-flags
    let sam_down = ctx.flag(flags::SAM_DOWN);
    sam_down.set(1).key("sam-down").await?; // bool or i32
    let value: i32 = sam_down.get().await?; // reads DCS (a mission trigger may change it too)
    // snippet:end skills-logic-flags
    ctx.report().set("sam_down", value);
    Ok(())
}

/// State documents in the three scopes, and the session report.
pub async fn state(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    // snippet:start skills-logic-state
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
    // snippet:end skills-logic-state
    Ok(())
}

/// What to do for each start kind.
pub async fn start_kinds(ctx: Ctx<Frontline>, start: Start) -> sortie::Result<()> {
    // snippet:start skills-logic-start-kinds
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
    // snippet:end skills-logic-start-kinds
    Ok(())
}
