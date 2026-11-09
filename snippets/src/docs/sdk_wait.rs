//! Waiting on time and on the world; reading the world replica.

use sortie::prelude::*;

use super::my_mission::{MyMission, groups, zones};

pub async fn sleep_and_timer(ctx: Ctx<MyMission>) {
    // snippet:start sdk-sleep-timer
    ctx.sleep(secs(30)).await; // 30 с модельного времени, пауза DCS его останавливает
    ctx.timer("red-cap-delay", mins(15)).await; // дедлайн хранится в движке
    // snippet:end sdk-sleep-timer
}

pub async fn wait_until(ctx: Ctx<MyMission>) {
    // snippet:start sdk-wait-until
    // Синий самолёт в зоне Target.
    ctx.wait_until(|w| {
        w.units()
            .coalition(Coalition::Blue)
            .air()
            .any_inside(zones::TARGET)
    })
    .await;
    // От бронегруппы осталось не больше двух машин.
    ctx.wait_until(|w| w.group(&groups::ARMOR).alive_count() <= 2)
        .await;
    // snippet:end sdk-wait-until
}

pub fn read_world(ctx: &Ctx<MyMission>) -> (usize, Option<String>, usize, Option<MapPoint>) {
    // snippet:start sdk-world
    let world = ctx.world(); // не держите его через .await
    let blue_players = world.players().coalition(Coalition::Blue).count();
    let nearest_red = world
        .units()
        .coalition(Coalition::Red)
        .ground()
        .nearest_to(zones::TARGET.center())
        .map(|unit| unit.name.clone());
    let blue_in_front = world
        .units()
        .coalition(Coalition::Blue)
        .count_inside(zones::FRONT);
    let armor = world.group(&groups::ARMOR);
    let armor_center = armor.center(); // Option<MapPoint>: середина живых машин
    // snippet:end sdk-world
    (blue_players, nearest_red, blue_in_front, armor_center)
}

pub fn clock(ctx: &Ctx<MyMission>) -> (f64, bool) {
    // snippet:start sdk-clock
    let now = ctx.model_time(); // секунды модельного времени
    let paused = ctx.paused(); // DCS на паузе
    // snippet:end sdk-clock
    (now, paused)
}

pub async fn session_end(ctx: Ctx<MyMission>) {
    // snippet:start sdk-session-end
    let end = ctx.until_session_end().await;
    eprintln!("сессия закончилась: {:?}, {:?}", end.outcome, end.reason);
    // snippet:end sdk-session-end
}
