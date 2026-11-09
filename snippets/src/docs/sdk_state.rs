//! State documents, run-once blocks, background tasks, the session's end.

use sortie::prelude::*;
use sortie::proto::v1::StateScope;

use super::my_mission::{MyMission, flags, groups};

// snippet:start sdk-state
#[derive(Default, Serialize, Deserialize)]
pub struct Campaign {
    pub targets_destroyed: u32,
}

pub async fn count_kill(ctx: &Ctx<MyMission>) -> sortie::Result<()> {
    let campaign = ctx.state::<Campaign>("campaign").await?; // область: кампания
    campaign.update(|c| c.targets_destroyed += 1).await?; // запись со сверкой ревизии
    let total = campaign.get().targets_destroyed;
    ctx.report().set("targets_destroyed", total);
    Ok(())
}
// snippet:end sdk-state

pub async fn scopes(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-state-scopes
    let seen = ctx
        .state_in::<Vec<String>>(StateScope::Session, "seen")
        .await?;
    seen.update(|names| names.push("Blue 1".into())).await?;
    let attempts = ctx.state_in::<u32>(StateScope::Turn, "attempts").await?;
    attempts.update(|n| *n += 1).await?;
    // snippet:end sdk-state-scopes
    Ok(())
}

pub async fn once(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-once
    // Один раз за сессию: перезапуск процесса логики не повторит.
    ctx.once("intro", async |ctx| {
        ctx.message(Coalition::Blue).text("Удачной охоты!").await
    })
    .await?;
    // Один раз за ход: не повторит и рестарт миссии.
    ctx.once_per_turn("target-down", async |ctx| {
        ctx.flag(flags::TARGET_DOWN).set(true).await
    })
    .await?;
    // snippet:end sdk-once
    Ok(())
}

pub async fn tasks(ctx: Ctx<MyMission>) {
    // snippet:start sdk-spawn-task
    let watcher = ctx.spawn(async |ctx| {
        loop {
            ctx.sleep(mins(5)).await;
            let left = ctx.world().group(&groups::ARMOR).alive_count();
            ctx.message(Coalition::Blue)
                .text(format!("Осталось машин: {left}"))
                .await?;
        }
    });
    // ...
    watcher.abort(); // необязательно: задачи и так заканчиваются вместе с сессией
    // snippet:end sdk-spawn-task
}

pub async fn end(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-end
    ctx.report().set("armor_left", 0);
    ctx.end_session(Outcome::BlueWin)
        .text("Бронегруппа уничтожена")
        .await
    // snippet:end sdk-end
}
