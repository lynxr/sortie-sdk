//! Examples of the `sortie-mission-logic` skill: DCS calls through `ctx.dcs()`
//! (references/api.md, "DCS calls").

use sortie::Error;
use sortie::dcs::{DcsObject, Handle};
use sortie::prelude::*;

use super::frontline::{Frontline, airbases, groups, zones};

// snippet:start skills-api-dcs
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
// snippet:end skills-api-dcs
