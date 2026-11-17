//! Examples of the `sortie-mission-logic` skill: F10 menu trees through `ctx.menus()`
//! (references/api.md, "Menu trees").

use sortie::prelude::*;
use sortie::sel::{Audience, GroupSel};

use super::frontline::{Frontline, zones};

// snippet:start skills-api-menu-trees
#[sortie::mission(api_rev = 1, event_rev = 1)] // trees: API rev 1; their presses: event rev 1
pub async fn run(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // One group's tree: SUSPENDED until a player takes a slot of "Viper 1"; the engine installs
    // it for the group's id, removes it when the slot empties and follows a dynamic slot.
    let jtac = ctx
        .menus()
        .tree("jtac/viper-1")
        .to(Audience::group(GroupSel::name("Viper 1")))
        .under(["JTAC"])
        .command("check-in", "Check in")
        .submenu("Smoke", |s| {
            s.command_arg("smoke-red", "Red", "red")
                .command_arg("smoke-green", "Green", "green")
        })
        .await?;
    let mut presses = jtac.presses();
    while let Some(p) = presses.next().await {
        if p.item.starts_with("smoke-") {
            let color = if p.arg == "red" {
                SmokeColor::Red
            } else {
                SmokeColor::Green
            };
            ctx.smoke(zones::OBJECTIVE.center(), color).await?;
        }
        if let Some(group) = p.group {
            ctx.message(group).text("Copy").await?;
        }
    }
    ctx.menus().remove("jtac/viper-1").await?;
    ctx.end_session(Outcome::Draw).await
}
// snippet:end skills-api-menu-trees
