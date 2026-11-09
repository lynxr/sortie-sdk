//! The event stream of a session.

use sortie::prelude::*;
use sortie::proto::v1;

use super::my_mission::{MyMission, groups};

pub async fn deaths(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-events
    let armor = ctx.group(groups::ARMOR)?;
    let mut deaths = ctx.events().of::<v1::Dead>().in_group(&armor);
    while let Some(dead) = deaths.next().await {
        let name = dead.initiator.map(|unit| unit.name).unwrap_or_default();
        ctx.message(Coalition::Blue)
            .text(format!("Уничтожен {name}"))
            .await?;
    }
    // snippet:end sdk-events
    Ok(())
}

pub async fn players(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-events-players
    let mut entered = ctx
        .events()
        .of::<v1::PlayerEnteredUnit>()
        .coalition(Coalition::Blue);
    while let Some(e) = entered.next().await {
        ctx.message(Coalition::Blue)
            .text(format!("{} занял слот", e.player))
            .await?;
    }
    // snippet:end sdk-events-players
    Ok(())
}

pub async fn hits(ctx: Ctx<MyMission>) {
    // snippet:start sdk-events-filter
    let mut hits = ctx.events().of::<v1::Hit>().filter(|hit| {
        hit.target
            .as_ref()
            .is_some_and(|target| target.type_name == "T-72B")
    });
    while let Some(hit) = hits.next().await {
        let weapon = hit.weapon.map(|w| w.type_name).unwrap_or_default();
        eprintln!("попадание в танк: {weapon}");
    }
    // snippet:end sdk-events-filter
}

pub async fn all_events(ctx: Ctx<MyMission>) {
    // snippet:start sdk-events-all
    let mut events = ctx.events();
    while let Some(event) = events.next().await {
        match event.kind {
            Some(v1::event::Kind::Takeoff(t)) => {
                eprintln!("взлёт: {:?}", t.initiator.map(|u| u.name));
            }
            Some(v1::event::Kind::Gap(_)) => eprintln!("часть событий потеряна"),
            _ => {}
        }
    }
    // snippet:end sdk-events-all
}
