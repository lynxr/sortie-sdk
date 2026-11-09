//! Handling `sortie::Error`.

use sortie::prelude::*;

use super::my_mission::{MyMission, airbases, templates};

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
