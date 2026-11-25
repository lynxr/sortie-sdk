//! Examples of the `sortie-mission-logic` skill: DCS calls through `ctx.dcs()`
//! (references/api.md, "DCS calls").

use sortie::Error;
use sortie::dcs::{DcsObject, Handle};
use sortie::prelude::*;

use super::frontline::{Frontline, groups, zones};

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
        .sounds()
        .out_sound("l10n/DEFAULT/alarm.ogg")
        .key("alarm-1")
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

/// A road path between two map points (`land.findPathOnRoads`, API revision 3, class heavy).
// snippet:start skills-api-road-path
#[sortie::mission(api_rev = 3)] // the road path is API revision 3
pub async fn convoy_distance(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    let armor = ctx.group(groups::RED_ARMOR)?;
    let objective = zones::OBJECTIVE.center();
    let Some(here) = ctx.world().group(&armor).center() else {
        return ctx.end_session(Outcome::Draw).await;
    };
    // Road type, then x (north) and y (east) of the start and of the end.
    let points = ctx
        .dcs()
        .land()
        .path_on_roads(
            "roads",
            here.north,
            here.east,
            objective.north,
            objective.east,
        )
        .await?; // Option<Vec<MapPoint>>, start to end; None: DCS answered nil
    // The length along the points; None (null in the report) when DCS found no path.
    let km: Option<f64> =
        points.map(|p| p.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>() / 1000.0);
    ctx.report().set("road_km_to_objective", km);
    ctx.end_session(Outcome::Draw).await
}
// snippet:end skills-api-road-path

/// The summary of a road path, computed by the Lua layer inside DCS (`sortie.findPathOnRoads`,
/// API revision 3, class heavy).
// snippet:start skills-api-road-summary
#[sortie::mission(api_rev = 3)] // the road path summary is API revision 3
pub async fn convoy_route(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    let armor = ctx.group(groups::RED_ARMOR)?;
    let objective = zones::OBJECTIVE.center();
    let Some(here) = ctx.world().group(&armor).center() else {
        return ctx.end_session(Outcome::Draw).await;
    };
    // Counted and measured inside DCS; at most 6 points travel: both ends, the sharpest turns.
    let summary = ctx
        .dcs()
        .land()
        .road_path("roads", here, objective, Some(6.0), None)
        .await?; // Option<RoadPath>; None: DCS found no path
    let Some(path) = summary else {
        ctx.report().set("road_km_to_objective", "no road");
        return ctx.end_session(Outcome::Draw).await;
    };
    ctx.report()
        .set("road_km_to_objective", path.length_m / 1000.0);
    ctx.report().set("road_points_in_dcs", path.count);
    // The route: the kept turns after the start, along the roads, if the road ends near.
    if path.points.len() > 1 && path.end_gap_m.is_some_and(|gap| gap < 500.0) {
        let legs = path.points[1..].iter().map(|&p| RoutePoint::on_road(p));
        armor
            .set_task(Task::ground_route(legs))
            .key("red-armor-road")
            .await?;
    }
    ctx.end_session(Outcome::Draw).await
}
// snippet:end skills-api-road-summary
