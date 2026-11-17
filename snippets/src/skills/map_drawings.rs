//! Examples of the `sortie-mission-logic` skill: drawings through `ctx.map()`
//! (references/api.md, "Map drawings").

use sortie::map::{Change, LineStyle, Rgba};
use sortie::prelude::*;
use sortie::proto::v1;

use super::frontline::{Frontline, zones};

// snippet:start skills-api-map
#[sortie::mission(api_rev = 1, event_rev = 1)] // drawings: API rev 1; ResourceEnded: event rev 1
pub async fn run(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    let obj = zones::OBJECTIVE.center();
    // A put by name: the same spec again sends nothing to DCS.
    let d = ctx
        .map()
        .circle("obj/ring", obj, 4_000.0)
        .color(Rgba::RED)
        .line_style(LineStyle::Dashed)
        .to(Coalition::Blue)
        .await?;
    assert!(matches!(d.change, Change::Created | Change::Unchanged));
    ctx.map()
        .panel("obj/label", obj, "Objective")
        .to(Coalition::Blue)
        .await?;
    // Ends that were not this logic's own remove.
    let mut ends = ctx.events().of::<v1::ResourceEnded>();
    if let Some(e) = ends.next().await {
        ctx.report().set("ended", e.name);
    }
    ctx.map().remove_prefix("obj/").await?;
    ctx.end_session(Outcome::Draw).await
}
// snippet:end skills-api-map
