//! Commands: messages, smoke, AI options, spawns, tasks, flags.

use sortie::prelude::*;

use super::my_mission::{MyMission, airbases, flags, groups, templates, zones};

pub async fn message_and_smoke(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-message
    ctx.message(Coalition::Blue)
        .text("Цель в 35 км к востоку от Кутаиси")
        .secs(20) // сколько секунд показывать (по умолчанию 10)
        .key("brief")
        .await?;
    ctx.smoke(zones::TARGET.center(), SmokeColor::Red).await?;
    // snippet:end sdk-message
    Ok(())
}

pub async fn options(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-options
    let sam = ctx.group(groups::SAM)?;
    sam.set_option(Alarm::Red).key("sam-red").await?;
    sam.set_option(Roe::WeaponFree)
        .key("sam-free")
        .timeout(secs(5)) // Error::Timeout, если DCS не ответила за 5 с
        .await?;
    // snippet:end sdk-options
    Ok(())
}

pub async fn spawn(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-spawn
    let cap = ctx
        .spawn_group(templates::RED_CAP.named("Red CAP 1"))
        .from_airbase(airbases::NALCHIK, StartType::Runway)
        .key("red-cap-1")
        .await?; // Group<Air>: группа уже в DCS
    let patrol = Task::cap(zones::PATROL)
        .altitude(ft(25_000))
        .speed(kts(420));
    cap.push_task(patrol).timeout(secs(5)).await?;
    // snippet:end sdk-spawn
    Ok(())
}

pub async fn spawn_at(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-spawn-at
    let point = zones::PATROL.center().alt(m(6_000)); // Vec3 над центром зоны
    let cap = ctx
        .spawn_group(templates::RED_CAP)
        .at(point, 0.0) // курс в радианах
        .key("red-cap-air")
        .await?;
    let orbit = Task::orbit(zones::PATROL.center())
        .race_track(zones::TARGET.center())
        .altitude(ft(20_000));
    cap.set_task(orbit).key("red-cap-orbit").await?;
    // ...
    cap.push_task(Task::rtb(airbases::NALCHIK)).await?;
    // snippet:end sdk-spawn-at
    Ok(())
}

pub async fn destroy(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-destroy
    let aaa = ctx.group(groups::AAA)?;
    aaa.destroy().key("aaa-removed").await?;
    // snippet:end sdk-destroy
    Ok(())
}

pub async fn flag(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-flags
    ctx.flag(flags::TARGET_DOWN).set(true).await?; // bool или i32
    let value: i32 = ctx.flag(flags::TARGET_DOWN).get().await?;
    // snippet:end sdk-flags
    let _ = value;
    Ok(())
}
