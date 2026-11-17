//! Drawings on the F10 map: the Draw resource through `ctx.map()`.

use std::time::Duration;

use sortie::map::{Change, LineStyle, Rgba};
use sortie::prelude::*;
use sortie::proto::v1;

use super::my_mission::{MyMission, zones};

// snippet:start sdk-map-mission
#[sortie::mission(api_rev = 1, event_rev = 1)] // рисунки — API ревизии 1, их концы — событий 1
pub async fn with_drawings(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    let target = zones::TARGET.center();
    ctx.map()
        .circle("zones/target", target, 3_000.0)
        .color(Rgba::RED)
        .fill(Rgba::RED.alpha(0.15))
        .line_style(LineStyle::Dashed)
        .to(Coalition::Blue)
        .await?;
    ctx.end_session(Outcome::Draw).await
}
// snippet:end sdk-map-mission

pub async fn shapes(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    let target = zones::TARGET.center();
    let a = MapPoint::new(target.north - 2_000.0, target.east);
    let b = MapPoint::new(target.north + 2_000.0, target.east + 1_000.0);
    // snippet:start sdk-map-shapes
    let map = ctx.map();
    map.line("front/1", a, b).color(Rgba::RED).await?;
    map.path("route", [a, target, b])
        .to(Coalition::Blue)
        .await?; // 2 линии DCS
    map.polygon("area", [a, target, b])
        .fill(Rgba::GREEN.alpha(0.2))
        .await?;
    map.rect("strip", a, b).await?;
    map.arrow("push", target, a).await?; // остриё в target
    map.text("label", b, "FLOT").font_size(24).await?;
    map.panel("rally", target, "Сбор здесь")
        .editable() // игроки могут менять и удалять
        .announce("Новая точка сбора")
        .await?;
    // snippet:end sdk-map-shapes
    Ok(())
}

pub async fn updates(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    let target = zones::TARGET.center();
    // snippet:start sdk-map-updates
    let zone = |r: f64| {
        ctx.map()
            .circle("zones/target", target, r)
            .color(Rgba::RED)
            .to(Coalition::Blue)
    };
    let d = zone(3_000.0).await?;
    assert_eq!(d.change, Change::Created);
    let d = zone(3_000.0).await?; // тот же спек: в DCS ничего не уходит
    assert_eq!((d.change, d.dcs_calls), (Change::Unchanged, 0));
    let d = zone(5_000.0).await?; // радиус меняется на месте, id метки тот же
    assert_eq!(d.change, Change::Updated);
    // Временная метка: исчезнет через 2 минуты модельного времени.
    ctx.map()
        .text("hint", target, "Цель накрыта")
        .ttl(Duration::from_secs(120))
        .await?;
    // snippet:end sdk-map-updates
    Ok(())
}

pub async fn ends(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-map-ends
    let mut ends = ctx.events().of::<v1::ResourceEnded>();
    let mut deleted_by_players = Vec::new();
    while let Some(e) = ends.next().await {
        if e.reason == v1::ResourceEndReason::Player as i32 {
            deleted_by_players.push(e.name); // игрок удалил редактируемую метку
        }
    }
    // Что зарегистрировано сейчас: реплика мира.
    let n = ctx.world().resources().count();
    ctx.map().remove_prefix("zones/").await?;
    ctx.map().clear().await?; // все рисунки этой логики
    // snippet:end sdk-map-ends
    let _ = (n, deleted_by_players);
    Ok(())
}
