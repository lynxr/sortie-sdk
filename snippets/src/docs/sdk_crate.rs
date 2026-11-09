//! A logic crate's `src/lib.rs` and the typed symbols of its scenario.

// snippet:start sdk-lib
use sortie::prelude::*;

sortie::include_scenario!(my_mission); // pub mod my_mission из scenario.lock.json
use my_mission::{MyMission, groups};

#[sortie::mission]
pub async fn run(ctx: Ctx<MyMission>, start: Start) -> sortie::Result<()> {
    if start == Start::Restart {
        ctx.message(Coalition::Blue)
            .text("Миссия перезапущена")
            .key("restarted")
            .await?;
    }
    let sam = ctx.group(groups::SAM)?; // Group<Ground>
    sam.set_option(Alarm::Red).key("sam-red").await?;
    ctx.until_session_end().await; // держать сессию до конца
    Ok(())
}
// snippet:end sdk-lib

// snippet:start sdk-symbols
use my_mission::{airbases, flags, menus, templates, zones};
use sortie::types::{AirbaseRef, FlagRef, GroupRef, MenuRef, TemplateRef, ZoneRef};

pub const SAM: GroupRef<MyMission, Ground> = groups::SAM;
pub const RED_CAP: TemplateRef<MyMission, Air> = templates::RED_CAP;
pub const TARGET: ZoneRef<MyMission> = zones::TARGET;
pub const NALCHIK: AirbaseRef<MyMission> = airbases::NALCHIK;
pub const TARGET_DOWN: FlagRef<MyMission> = flags::TARGET_DOWN;
pub const SMOKE_TARGET: MenuRef<MyMission> = menus::SMOKE_TARGET;
// snippet:end sdk-symbols
