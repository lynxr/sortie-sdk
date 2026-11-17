//! F10 menu trees: the MenuTree resource through `ctx.menus()`.

use sortie::map::Change;
use sortie::menu_tree::Overflow;
use sortie::prelude::*;
use sortie::sel::{Audience, GroupSel};

use super::my_mission::{MyMission, menus, zones};

// snippet:start sdk-menus-mission
#[sortie::mission(api_rev = 1, event_rev = 1)] // деревья меню — API ревизии 1, нажатия — событий 1
pub async fn with_menus(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    let support = ctx
        .menus()
        .tree("support")
        .to(Coalition::Blue)
        .under(["Поддержка"])
        .command("sitrep", "Обстановка")
        .submenu("Дым", |s| {
            s.command_arg("smoke-red", "Красный", "red").command_arg(
                "smoke-green",
                "Зелёный",
                "green",
            )
        })
        .await?;
    let mut presses = support.presses();
    while let Some(p) = presses.next().await {
        if p.item.starts_with("smoke-") {
            let color = if p.arg == "red" {
                SmokeColor::Red
            } else {
                SmokeColor::Green
            };
            ctx.smoke(zones::TARGET.center(), color).await?;
        }
    }
    ctx.end_session(Outcome::Draw).await
}
// snippet:end sdk-menus-mission

pub async fn group_tree(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-menus-group
    let jtac = ctx
        .menus()
        .tree("jtac/blue-1")
        .to(Audience::group(GroupSel::name("Blue 1")))
        .under(["JTAC"])
        .command("check-in", "Прибыл в район")
        .manifest_command("smoke", menus::SMOKE_TARGET) // надпись из сценария
        .await?;
    if jtac.is_suspended() {
        // В группе пока нет игрока: меню появится, когда игрок займёт слот.
        ctx.report().set("jtac", "waiting");
    }
    ctx.spawn(async move |ctx| {
        let mut presses = jtac.presses();
        while let Some(p) = presses.next().await {
            if let Some(group) = p.group {
                ctx.message(group)
                    .text(format!("Принято: {}", p.item))
                    .await?;
            }
        }
        Ok(())
    });
    // snippet:end sdk-menus-group
    Ok(())
}

pub async fn updates(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-menus-updates
    let tree = |extra: bool| {
        let t = ctx
            .menus()
            .tree("support")
            .to(Coalition::Blue)
            .command("sitrep", "Обстановка");
        if extra {
            t.command("tanker", "Где заправщик?")
        } else {
            t
        }
    };
    let m = tree(false).await?;
    assert_eq!(m.change, Change::Created);
    let m = tree(false).await?; // то же дерево: в DCS ничего не уходит
    assert_eq!((m.change, m.dcs_calls), (Change::Unchanged, 0));
    let m = tree(true).await?; // добавлен пункт в конце: один вызов DCS
    assert_eq!((m.change, m.dcs_calls), (Change::Updated, 1));
    ctx.menus().remove("support").await?;
    // snippet:end sdk-menus-updates
    Ok(())
}

pub async fn pages(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-menus-pages
    // 14 целей на корневом уровне дерева: DCS покажет 9 и «More... (targets)», а в нём
    // остальные 5.
    let targets = (1..=14).fold(
        ctx.menus()
            .tree("targets")
            .to(Coalition::Blue)
            .under(["Цели"]),
        |t, i| t.command_arg(&format!("target-{i}"), &format!("Цель {i}"), &i.to_string()),
    );
    let m = targets.await?;
    assert_eq!((m.change, m.dcs_calls), (Change::Created, 15)); // 14 пунктов и 1 страница
    let mut presses = m.presses();
    if let Some(p) = presses.next().await {
        // Нажатие «Цель 12» на странице несёт её id и arg: "target-12", "12".
        ctx.report().set("target", &p.item);
    }
    // Без листания: больше 10 пунктов на уровне — Error::Invalid, ничего не меняется.
    let strict = (1..=11).fold(
        ctx.menus().tree("strict").overflow(Overflow::Reject),
        |t, i| t.command(&format!("s-{i}"), &format!("Пункт {i}")),
    );
    assert!(matches!(strict.await, Err(sortie::Error::Invalid(_))));
    // snippet:end sdk-menus-pages
    Ok(())
}
