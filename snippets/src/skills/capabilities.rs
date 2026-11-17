//! Examples of the `sortie-mission-logic` skill: capabilities (references/api.md,
//! "Capabilities").

use sortie::caps;
use sortie::prelude::*;

use super::frontline::Frontline;

// snippet:start skills-api-caps
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
// snippet:end skills-api-caps
