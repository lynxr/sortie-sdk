//! Логика миссии my_mission. `run` вызывается на каждую сессию: при загрузке миссии в DCS, после
//! её рестарта и после перезапуска процесса логики.

use sortie::prelude::*;

sortie::include_scenario!(my_mission); // модуль из scenario.lock.json (см. build.rs)
use my_mission::{MyMission, groups, zones};

#[sortie::mission]
pub async fn run(ctx: Ctx<MyMission>, start: Start) -> sortie::Result<()> {
    let armor = ctx.group(groups::ARMOR)?; // Group<Ground>: группа есть в мире
    let sam = ctx.group(groups::SAM)?;

    if start.is_fresh() {
        ctx.message(Coalition::Blue)
            .text("Цель: бронегруппа в зоне Target. Её прикрывают «Оса» и «Шилки».")
            .secs(20)
            .key("intro")
            .await?;
    }

    // ЗРК молчит, пока синие самолёты не подойдут к цели ближе 15 км.
    sam.set_option(Alarm::Green).key("sam-green").await?;
    ctx.wait_until(|w| {
        w.units()
            .coalition(Coalition::Blue)
            .air()
            .within(zones::TARGET.center(), km(15))
            .any()
    })
    .await;
    if sam.is_alive() {
        sam.set_option(Alarm::Red).key("sam-red").await?;
        ctx.message(Coalition::Blue)
            .text("«Оса» включила радар!")
            .key("sam-alert")
            .await?;
    }

    // Победа, когда от бронегруппы ничего не осталось.
    ctx.wait_until(|w| w.group(&armor).alive_count() == 0).await;
    ctx.message(Coalition::Blue)
        .text("Цель уничтожена. Миссия выполнена!")
        .secs(30)
        .key("win")
        .await?;
    ctx.end_session(Outcome::BlueWin)
        .text("бронегруппа уничтожена")
        .await
}
