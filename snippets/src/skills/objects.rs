//! Examples of the `sortie-mission-logic` skill: static objects, destroy and junk
//! (references/api.md, "Objects").

use sortie::Error;
use sortie::cmd::{Heliport, OnConflict};
use sortie::prelude::*;
use sortie::sel::{Obj, Volume};

use super::frontline::{Frontline, airbases, templates, zones};

// snippet:start skills-api-objects
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
// snippet:end skills-api-objects

pub async fn restore(ctx: Ctx<Frontline>, depots: Vec<(String, MapPoint)>) -> sortie::Result<()> {
    // snippet:start skills-api-batch
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
    // snippet:end skills-api-batch
    let _ = cap;
    Ok(())
}
