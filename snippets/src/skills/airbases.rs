//! Examples of the `sortie-mission-logic` skill: the owner, auto capture and radio silence of
//! airbases, the owner event and the airbase queries (references/api.md, "Airbases").

use sortie::Error;
use sortie::cmd::TerminalType;
use sortie::prelude::*;
use sortie::proto::v1;
use sortie::sel::AirbaseSel;

use super::frontline::{Frontline, airbases};

// snippet:start skills-api-airbases
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
// snippet:end skills-api-airbases

// snippet:start skills-api-airbase-owners
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
// snippet:end skills-api-airbase-owners

// snippet:start skills-api-airbase-queries
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
// snippet:end skills-api-airbase-queries
