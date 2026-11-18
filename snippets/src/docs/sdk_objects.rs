//! Objects (API revision 1): static objects, destroying any object, removing junk, batches.

use sortie::Error;
use sortie::cmd::{Heliport, OnConflict, SpawnedStatic};
use sortie::prelude::*;
use sortie::sel::{Obj, Volume};

use super::my_mission::{MyMission, airbases, templates, zones};

// snippet:start sdk-objects-statics
#[sortie::mission(api_rev = 1)] // статики, destroy и remove_junk — API ревизии 1
pub async fn statics(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    let p = zones::TARGET.center(); // MapPoint
    // Категорию и модель DCS движок берёт из своей таблицы типов.
    let depot = ctx
        .spawn_static(".Ammunition depot", "USA")
        .named("Depot North")
        .at(p, 0.0)
        .key("depot-north")
        .await?;
    // Статик уже в реплике мира; DCS не шлёт BIRTH для статиков, событие Birth делает движок.
    let alive = ctx.world().entity(depot.eid).is_some_and(|e| e.is_alive());
    // Рабочий FARP — аэродром с позывным и частотой.
    let farp = ctx
        .spawn_static("FARP", "USA")
        .named("FARP North")
        .at(p.toward(90.0_f64.to_radians(), m(500)), 0.0)
        .heliport(
            Heliport::new()
                .callsign("London")
                .frequency_hz(127_500_000.0),
        )
        .key("farp-north")
        .await?;
    // Груз, который может поднять вертолёт; имя по умолчанию — `ammo_cargo#<сессия>-<n>`.
    let crate_ = ctx
        .spawn_static("ammo_cargo", "USA")
        .cargo(800.0, true)
        .at(p.toward(0.0, m(50)), 0.0)
        .key("crate-1")
        .await?;
    ctx.report().set("farp_airbase", farp.airbase.is_some());
    ctx.report().set("crate", crate_.name);
    let _ = alive;
    ctx.end_session(Outcome::Draw).await
}
// snippet:end sdk-objects-statics

pub async fn conflicts(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    let p = zones::TARGET.center();
    // snippet:start sdk-objects-conflicts
    // Имя — личность статика. Повтор после перезапуска логики:
    let depot = ctx
        .spawn_static(".Ammunition depot", "USA")
        .named("Depot North")
        .at(p, 0.0)
        .on_conflict(OnConflict::Keep) // живой статик того же типа — он и есть результат
        .key("depot-north-again")
        .await?;
    if depot.kept {
        // ничего не создано
    }
    // Заменить живой статик новым (игрок в статике не сидит):
    ctx.spawn_static(".Command Center", "USA")
        .named("Depot North")
        .at(p, 0.0)
        .on_conflict(OnConflict::Replace) // старый получает UnitGone с причиной Replaced
        .key("depot-north-hq")
        .await?;
    // snippet:end sdk-objects-conflicts
    Ok(())
}

pub async fn destroy_and_junk(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-objects-destroy
    ctx.destroy(Obj::static_object("Depot North"))
        .key("depot-north-gone")
        .await?;
    // Юнит с игроком не уничтожается без .allow_player(): у клиентов остаются «призраки».
    match ctx.destroy(Obj::unit("Viper 1-1")).key("viper-gone").await {
        Ok(()) | Err(Error::Gone(_)) => {}
        Err(Error::Rejected(_)) => ctx.report().set("viper", "occupied by a player"),
        Err(e) => return Err(e),
    }
    // Обломки, воронки и мусор в объёмах DCS; результат — что насчитала DCS.
    let center = zones::TARGET.center();
    let junk = ctx
        .remove_junk([Volume::sphere_on_ground(center, 2_000.0)])
        .await?;
    ctx.report().set("junk_removed", junk.total);
    // snippet:end sdk-objects-destroy
    Ok(())
}

pub async fn batch(ctx: Ctx<MyMission>, saved: &[(String, MapPoint)]) -> sortie::Result<()> {
    // snippet:start sdk-objects-batch
    // Восстановление кампании: одна команда, один ключ; движок проверяет весь пакет сразу.
    let mut batch = ctx.spawn_batch();
    let cap = batch.add_group(
        ctx.spawn_group(templates::RED_CAP.named("Red CAP 1"))
            .from_airbase(airbases::NALCHIK, StartType::Runway),
    );
    let farp = batch.add_static(
        ctx.spawn_static("FARP", "USA")
            .named("FARP North")
            .at(zones::TARGET.center(), 0.0)
            .heliport(Heliport::new()),
    );
    for (name, p) in saved {
        batch = batch.static_object(
            ctx.spawn_static(".Ammunition depot", "USA")
                .named(name.as_str())
                .at(*p, 0.0)
                .on_conflict(OnConflict::Keep),
        );
    }
    // Ограничитель спавна раздаёт элементы по времени: ответ приходит после последнего.
    let out = batch.key("restore-turn-7").await?;
    let cap: Group<Air> = out.get(&cap)?; // типизированный результат элемента
    let farp: SpawnedStatic = out.get(&farp)?;
    for item in out.items.iter().filter(|i| !i.is_ok()) {
        // Ничего не откатывается: у каждого элемента свой результат.
        ctx.report().set(
            item.name.as_str(),
            item.error().map_or("skipped", |e| e.code()),
        );
    }
    // snippet:end sdk-objects-batch
    let _ = (cap, farp);
    Ok(())
}

// snippet:start sdk-objects-search
#[sortie::mission(api_rev = 2)] // ctx.search_objects — API ревизии 2
pub async fn search(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    use sortie::cmd::ObjectCategory as C;
    let p = zones::TARGET.center();
    // Шар радиусом 300 м на земле: юниты, статики и объекты карты (по умолчанию).
    let found = ctx
        .search_objects(Volume::sphere_on_ground(p, 300.0))
        .categories([C::Unit, C::Static, C::Scenery])
        .await?;
    for o in &found.objects {
        // Юнит или статик из мира — с его Eid; у объекта карты Eid нет, есть его id в DCS.
        let intact = match (o.life, o.life_max) {
            (Some(life), Some(max)) if max > 0.0 => life / max,
            _ => 1.0,
        };
        let in_world = o
            .eid
            .and_then(|eid| ctx.world().entity(eid).map(|e| e.is_alive()));
        let _ = (intact, in_world, o.dcs_id, &o.type_name);
    }
    // Больше лимита (по умолчанию 500, не больше 2000) — `truncated`, остальное не прочитано.
    let busy = found.truncated || !found.objects.is_empty();
    // snippet:end sdk-objects-search
    let _ = busy;
    Ok(())
}
