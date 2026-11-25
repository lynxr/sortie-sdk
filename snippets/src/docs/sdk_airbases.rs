//! Airbases (API revision 3): owner, DCS's auto capture and radio silence of many airbases in
//! one command; the owner event (event revision 2); the queries of parking, runways and the
//! control state.

use sortie::Error;
use sortie::cmd::{Heliport, TerminalType};
use sortie::prelude::*;
use sortie::proto::v1;
use sortie::sel::AirbaseSel;

use super::my_mission::{MyMission, airbases, zones};

// snippet:start sdk-airbases-control
#[sortie::mission(api_rev = 3)] // ctx.airbase_control — API ревизии 3
pub async fn front_moved(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    // Оба аэродрома — красным, автозахват DCS выключен, диспетчеры молчат. Всё — за один кадр.
    let out = ctx
        .airbase_control([airbases::NALCHIK, airbases::KUTAISI])
        .owner(Coalition::Red)
        .auto_capture(false) // применяется раньше владельца: DCS не вернёт базу сама
        .radio_silent(true)
        .key("front-moved-1")
        .await?;
    // Мир знает нового владельца сразу, как команда вернулась.
    let red = ctx
        .world()
        .airbase_by_name("Kutaisi")
        .is_some_and(|a| a.coalition == Some(Coalition::Red));
    // У каждой базы свой результат: что DCS прочитала до и после, или её ошибка.
    for base in &out.bases {
        match base.error() {
            None => {
                let after = base.after.and_then(|s| s.owner);
                ctx.report().set(&base.name, format!("{after:?}"));
            }
            // DCS не нашла базу (Error::Gone) или сеттер упал (Error::Lua) — только у неё.
            Some(e) => ctx.report().set(&base.name, e.code()),
        }
    }
    ctx.report()
        .set("all_ok", out.ok_count() == out.bases.len() && red);
    ctx.end_session(Outcome::Draw).await
}
// snippet:end sdk-airbases-control

pub async fn farp_and_refusals(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    let p = zones::TARGET.center();
    // snippet:start sdk-airbases-farp
    // FARP — тоже аэродром: по имени, по Eid аэродрома или по Eid его статика.
    let farp = ctx
        .spawn_static("FARP", "USA")
        .named("FARP North")
        .at(p, 0.0)
        .heliport(Heliport::new())
        .key("farp-north")
        .await?;
    ctx.airbase_control([AirbaseSel::eid(farp.eid), AirbaseSel::name("Nalchik")])
        .radio_silent(false) // только радио; владелец и автозахват не меняются
        .key("farp-north-radio")
        .await?;
    // Базы, которой нет в мире, команда не отправляет вовсе: Error::Invalid с её именем.
    match ctx
        .airbase_control([AirbaseSel::name("Batumi")])
        .owner(Coalition::Blue)
        .await
    {
        Err(Error::Invalid(f)) => ctx.report().set("refused", f.message.clone()),
        other => {
            other?;
        }
    }
    // snippet:end sdk-airbases-farp
    Ok(())
}

// snippet:start sdk-airbases-owner-events
#[sortie::mission(api_rev = 3, event_rev = 2)] // AirbaseOwnerChanged — событие ревизии 2
pub async fn owner_events(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    // Одно событие на каждую смену владельца — что бы о ней ни сообщило движку.
    let mut owners = ctx.events().of::<v1::AirbaseOwnerChanged>();
    while let Some(e) = owners.next().await {
        let name = e
            .airbase
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_default();
        // Мир уже знает нового владельца, когда событие пришло.
        let in_world = ctx.world().airbase_by_name(&name).and_then(|a| a.coalition);
        let why = match e.cause() {
            v1::OwnerChangeCause::Captured => "захват DCS", // S_EVENT_BASE_CAPTURED
            v1::OwnerChangeCause::Control => "airbase_control",
            v1::OwnerChangeCause::Discovered => "найдено при чтении DCS",
            v1::OwnerChangeCause::Unspecified => "?",
        };
        // previous() — Unspecified, если прежнего владельца движок не знал.
        ctx.report().set(
            &name,
            format!(
                "{:?} -> {:?} ({why}), мир: {in_world:?}",
                e.previous(),
                e.owner()
            ),
        );
    }
    ctx.end_session(Outcome::Draw).await
}
// snippet:end sdk-airbases-owner-events

pub async fn owner_filters(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-airbases-owner-filters
    // Только захваты Кутаиси самой DCS (наземными войсками).
    let mut kutaisi = ctx
        .events()
        .of::<v1::AirbaseOwnerChanged>()
        .airbase(airbases::KUTAISI)
        .cause(v1::OwnerChangeCause::Captured);
    // Всё, что перешло к красным: coalition() — новый владелец.
    let mut to_red = ctx
        .events()
        .of::<v1::AirbaseOwnerChanged>()
        .coalition(Coalition::Red);
    if let Some(e) = kutaisi.next().await {
        ctx.message(Coalition::Blue)
            .text(format!("Кутаиси захвачен: {:?}", e.owner()))
            .await?;
    }
    let _lost = to_red.next().await;
    // snippet:end sdk-airbases-owner-filters
    Ok(())
}

// snippet:start sdk-airbases-layout
#[sortie::mission(api_rev = 3)] // запросы аэродромов — API ревизии 3
pub async fn place_aircraft(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    // Свободные стоянки Кутаиси: только те, что DCS отдаёт как свободные.
    let free = ctx
        .airbase(airbases::KUTAISI)
        .parking()
        .available_only()
        .await?;
    // Тип стоянки — по имени; число, которого SDK не знает, — TerminalType::Unknown.
    let open = free
        .iter()
        .filter(|s| {
            matches!(
                s.terminal,
                Some(TerminalType::OpenAir | TerminalType::OpenOrShelter)
            )
        })
        .count();
    // Все стоянки: какие заняты и кем (Eid мира, если DCS назвала объект и мир его держит).
    let all = ctx.airbase(airbases::KUTAISI).parking().await?;
    let taken = all.iter().filter(|s| s.taken == Some(true)).count();
    // Полосы: название, курс (радианы, как отдаёт DCS), длина и ширина в метрах, центр.
    for rw in ctx.airbase(airbases::KUTAISI).runways().await? {
        ctx.report().set(
            &format!("runway {}", rw.name),
            format!(
                "{:?} deg, {:?} m",
                rw.heading_deg().map(f64::round),
                rw.length
            ),
        );
    }
    ctx.report().set("open", open);
    ctx.report().set("taken", taken);
    ctx.end_session(Outcome::Draw).await
}
// snippet:end sdk-airbases-layout

pub async fn read_many(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-airbases-batch
    // Владелец, автозахват и радио многих баз — одним запросом (DCS читает 32 базы за кадр).
    let read = ctx
        .airbases_state([airbases::KUTAISI, airbases::NALCHIK])
        .await?;
    for base in &read.bases {
        match (base.value, base.error()) {
            (Some(s), _) => ctx.report().set(&base.name, format!("{:?}", s.owner)),
            // У каждой базы своя ошибка: Error::Gone — базы нет в DCS или в мире она мертва.
            (None, Some(e)) => ctx.report().set(&base.name, e.code()),
            (None, None) => {}
        }
    }
    // Стоянки до 16 баз одним запросом; база, не поместившаяся в ответ, — Error::Rejected.
    let parking = ctx
        .airbases_parking([AirbaseSel::name("Kutaisi"), AirbaseSel::name("FARP North")])
        .available_only()
        .await?;
    ctx.report().set("bases_ok", parking.ok_count());
    // snippet:end sdk-airbases-batch
    Ok(())
}
