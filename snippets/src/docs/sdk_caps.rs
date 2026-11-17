//! Capabilities: `requires = [..]`, `ctx.capabilities()`, `Error::capability()`.

use sortie::caps;
use sortie::prelude::*;
use sortie::proto::v1;

use super::my_mission::MyMission;

// snippet:start sdk-caps-requires
#[sortie::mission(api_rev = 1, requires = [caps::SPAWN_STATIC, caps::DRAW])]
pub async fn needs_statics(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    // Сюда логика попадает, только если обе возможности AVAILABLE: иначе движок отказал в
    // Attach (AttachRejected.missing_capabilities), и миссия-функция не запускалась.
    ctx.end_session(Outcome::Draw).await
}
// snippet:end sdk-caps-requires

pub async fn table(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-caps-table
    let t = ctx.capabilities();
    let junk = t.is_available(caps::REMOVE_JUNK);
    // Почему нет (MISSING_IN_DCS, DISABLED, QUARANTINED...): заметка движка.
    let why = t.note(caps::REMOVE_JUNK).to_owned();
    let height = t.state(caps::call::mission::LAND_GET_HEIGHT);
    let quarantined: Vec<&str> = t
        .in_state(v1::CapabilityState::Quarantined)
        .map(|c| c.name.as_str())
        .collect();
    // snippet:end sdk-caps-table
    let _ = (junk, why, height, quarantined);
    // snippet:start sdk-caps-one
    if let Some(c) = ctx.capability(caps::MENU_TREE) {
        // Состояние, заметка, id живых проверок (PLY-LC1, ...), с какой ревизии API.
        let _ = (c.state, c.note, c.live_checks, c.since_api_rev);
    }
    // snippet:end sdk-caps-one
    Ok(())
}

pub async fn refused(ctx: Ctx<MyMission>) -> sortie::Result<Option<f64>> {
    // snippet:start sdk-caps-error
    match ctx.dcs().land().height(MapPoint::new(0.0, 0.0)).await {
        Ok(h) => Ok(Some(h)),
        // Этот путь на карантине после падения DCS: capability() = Some("call:mission:...").
        Err(e @ sortie::Error::Quarantined(_))
            if e.capability() == Some(caps::call::mission::LAND_GET_HEIGHT) =>
        {
            Ok(None)
        }
        Err(e) => Err(e),
    }
    // snippet:end sdk-caps-error
}
