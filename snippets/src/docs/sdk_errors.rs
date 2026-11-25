//! Handling `sortie::Error`.

use sortie::prelude::*;

use super::my_mission::{MyMission, airbases, groups, templates};

pub async fn send_home(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    let cap = ctx.spawn_group(templates::RED_CAP).key("red-cap").await?;
    // snippet:start sdk-errors
    match cap.push_task(Task::rtb(airbases::NALCHIK)).await {
        Ok(()) => {}
        Err(sortie::Error::Gone(_)) => {} // группы уже нет: её сбили
        Err(e) => {
            eprintln!("{}", e.report()); // место в коде, Lua traceback, подсказка
            return Err(e);
        }
    }
    // snippet:end sdk-errors
    Ok(())
}

pub async fn optional_option(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-errors-timeout
    let sam = ctx.group(groups::SAM)?;
    let go_red = || sam.set_option(Alarm::Red).key("sam-alarm-red");
    let mut outcome = go_red().await;
    if matches!(outcome, Err(sortie::Error::Timeout(_))) {
        // С тем же ключом: команда, которая всё же дошла до DCS, второй раз не выполнится.
        outcome = go_red().await;
    }
    if let Err(e) = outcome {
        // Необязательная команда: записать и продолжать, не завершая функцию миссии.
        eprintln!("{}", e.report());
    }
    // snippet:end sdk-errors-timeout
    Ok(())
}
