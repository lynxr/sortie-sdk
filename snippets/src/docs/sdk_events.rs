//! The event stream of a session.

use sortie::Error;
use sortie::prelude::*;
use sortie::proto::v1;
use sortie::sel::Obj;

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

pub async fn dcs_entries(ctx: Ctx<MyMission>) {
    // snippet:start sdk-events-dcs
    // Только обломки: гибель пилота DCS тоже присылает как Dead, а самолёт ещё летит.
    let mut wrecks = ctx
        .events()
        .of::<v1::Dead>()
        .from_dcs(v1::DcsEvent::Dead)
        .from_dcs(v1::DcsEvent::Crash);
    while let Some((dead, entry)) = wrecks.next_with_dcs_event().await {
        let name = dead.initiator.map(|u| u.name).unwrap_or_default();
        eprintln!("{name}: {entry:?}");
    }
    // snippet:end sdk-events-dcs
}

pub async fn generic_form(ctx: Ctx<MyMission>) {
    // snippet:start sdk-events-generic
    let mut starts = ctx
        .events()
        .of::<v1::Unknown>()
        .from_dcs(v1::DcsEvent::EngineStartup)
        .coalition(Coalition::Blue);
    while let Some(start) = starts.next().await {
        let ctx_ = start.ctx.unwrap_or_default();
        let unit = ctx_.initiator.map(|u| u.name).unwrap_or_default();
        // Ключи DCS без своего поля — в `extra`, как DCS их прислал.
        let sub = ctx_
            .extra
            .as_ref()
            .and_then(|x| x.get("subPlace"))
            .and_then(|v| v.as_f64());
        eprintln!("{unit} запустил двигатели (subPlace {sub:?})");
    }
    // snippet:end sdk-events-generic
}

#[sortie::mission(api_rev = 1)] // Destroy.object и Eid оружия — API ревизии 1
pub async fn weapons(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    // snippet:start sdk-events-weapons
    let mut shots = ctx.events().of::<v1::Shot>();
    let mut hits = ctx.events().of::<v1::Hit>();
    let Some(shot) = shots.next().await else {
        return Ok(());
    };
    let missile = shot.weapon.unwrap_or_default();
    // Тот же Eid в Hit и Kill этой ракеты.
    while let Some(hit) = hits.next().await {
        if hit.weapon.as_ref().is_some_and(|w| w.eid == missile.eid) {
            eprintln!("{} попала", missile.type_name);
            break;
        }
    }
    // Следующую ракету — убрать в полёте (учебная тревога).
    if let Some(next) = shots.next().await
        && let Some(obj) = next.weapon.as_ref().and_then(Obj::weapon)
    {
        match ctx.destroy(obj).key("drill-missile").await {
            Ok(()) | Err(Error::Gone(_)) => {} // Gone: уже взорвалась
            Err(e) => return Err(e),
        }
    }
    // snippet:end sdk-events-weapons
    Ok(())
}
