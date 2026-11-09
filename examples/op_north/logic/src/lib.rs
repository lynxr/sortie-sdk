// examples/op_north/logic/src/lib.rs
use sortie::prelude::*;

sortie::include_scenario!(op_north);                  // сгенерировано из scenario.lock.json
use op_north::{airbases, flags, groups, menus, templates, zones, OpNorth};

#[derive(Default, Serialize, Deserialize)]
struct Campaign { sams_killed: u32 }

#[sortie::mission]                                    // вызывается на каждую сессию op_north
pub async fn run(ctx: Ctx<OpNorth>, start: Start) -> sortie::Result<()> {
    let campaign = ctx.state::<Campaign>("campaign").await?;  // хранится в движке
    let sam = ctx.group(groups::SAM_NORTH)?;                   // Group<Ground>, сверено с manifest
    sam.set_option(Alarm::Green).key("sam-dark").await?;       // key: идемпотентно при reload

    if start.is_fresh() {
        ctx.message(Coalition::Blue).text("SA-10 North is cold. Push when ready.")
            .secs(20).key("intro").await?;
    }

    // Меню у каждой синей группы (addCommandForGroup на BIRTH), поэтому видно, кто нажал.
    let smoke = ctx.menu(Coalition::Blue).per_group()
        .command(menus::SMOKE_OBJECTIVE).key("menu-smoke").await?;
    ctx.spawn(async move |ctx| {
        let mut presses = smoke.presses();
        while let Some(p) = presses.next().await {
            ctx.smoke(zones::OBJECTIVE.center(), SmokeColor::Red).await?;
            ctx.message(p.group).text("Red smoke on the objective").secs(10).await?;
        }
        Ok(())
    });

    // Условие проверяется на локальной реплике каждый тик, без RPC.
    ctx.wait_until(|w| w.units().coalition(Coalition::Blue).air().any_inside(zones::SAM_RING)).await;
    sam.set_option(Alarm::Red).key("sam-hot").await?;

    // Ждём состояние («живых не осталось»), а не событие: корректно и после hot reload.
    ctx.wait_until(|w| w.group(&sam).alive_count() == 0).await;
    ctx.once_per_turn("sam-north-down", async |ctx| {          // кампания: один раз за ход
        campaign.update(|c| c.sams_killed += 1).await?;
        ctx.flag(flags::SAM_NORTH_DOWN).set(true).await
    }).await?;

    ctx.timer("red-cap-delay", secs(90)).await;             // дедлайн хранится в движке
    let cap = ctx.spawn_group(templates::RED_CAP.named("Red CAP 1"))
        .from_airbase(airbases::MAYKOP_KHANSKAYA, StartType::Runway)
        .key("red-cap-1").await?;                          // в dev-режиме спавн без key — ошибка
    cap.push_task(Task::cap(zones::SAM_RING).altitude(ft(25_000)).speed(kts(420)))
        .timeout(secs(5)).await?;

    // Ход кончается, когда CAP сбит; иначе его закроет лимит времени из manifest.
    ctx.wait_until(|w| w.group(&cap).alive_count() == 0).await;
    ctx.report().set("sams_killed", campaign.get().sams_killed);
    ctx.end_session(Outcome::BlueWin).await                 // worker генератора соберёт следующий ход
}
