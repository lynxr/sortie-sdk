//! F10 radio menus.

use sortie::prelude::*;

use super::my_mission::{MyMission, menus, zones};

pub async fn coalition_menu(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-menu
    let info = ctx
        .menu(Coalition::Blue)
        .command(menus::TARGET_INFO)
        .key("menu-info")
        .await?;
    let mut presses = info.presses();
    while presses.next().await.is_some() {
        ctx.message(Coalition::Blue)
            .text("Цель: зона Target, 35 км восточнее Кутаиси")
            .await?;
    }
    // snippet:end sdk-menu
    Ok(())
}

pub async fn per_group_menu(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-menu-per-group
    let smoke = ctx
        .menu(Coalition::Blue)
        .per_group()
        .command(menus::SMOKE_TARGET)
        .key("menu-smoke")
        .await?;
    ctx.spawn(async move |ctx| {
        let mut presses = smoke.presses();
        while let Some(press) = presses.next().await {
            ctx.smoke(zones::TARGET.center(), SmokeColor::Red).await?;
            ctx.message(press.group) // ответ только той группе, что нажала
                .text("Красный дым на цели")
                .await?;
        }
        Ok(())
    });
    // snippet:end sdk-menu-per-group
    Ok(())
}

pub async fn remove_menu(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    let info = ctx
        .menu(Coalition::Blue)
        .command(menus::TARGET_INFO)
        .await?;
    // snippet:start sdk-menu-remove
    info.remove().await?;
    // snippet:end sdk-menu-remove
    Ok(())
}
