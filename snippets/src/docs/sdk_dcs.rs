//! The universal call: DCS functions of the DCS API catalog through `ctx.dcs()`.

use sortie::Error;
use sortie::dcs::{DcsObject, Handle, Value};
use sortie::prelude::*;
use sortie::sel::{GroupSel, UnitSel};

use super::my_mission::{MyMission, airbases, groups, zones};

// snippet:start sdk-dcs-mission
#[sortie::mission(api_rev = 1)] // вызовы DCS — API ревизии 1
pub async fn with_calls(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    let height = ctx.dcs().land().height(zones::TARGET.center()).await?;
    ctx.report().set("target_height_m", height);
    ctx.end_session(Outcome::Draw).await
}
// snippet:end sdk-dcs-mission

pub async fn reads(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-dcs-reads
    let dcs = ctx.dcs();
    let p = zones::TARGET.center(); // MapPoint
    let surface: String = dcs.land().surface_type(p).await?; // "LAND", "ROAD", "WATER", ...
    let height: f64 = dcs.land().height(p).await?;
    let (kelvin, pascal) = dcs
        .atmosphere()
        .temperature_and_pressure(p.alt(m(3_000)))
        .await?;
    let owner: Coalition = dcs.airbase(airbases::KUTAISI).coalition().await?;
    // snippet:end sdk-dcs-reads
    let _ = (surface, height, kelvin, pascal, owner);
    Ok(())
}

// snippet:start sdk-dcs-road-path
#[sortie::mission(api_rev = 3)] // land().path_on_roads — API ревизии 3
pub async fn road_path(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    let armor = ctx.group(groups::ARMOR)?;
    let start = ctx.world().group(&armor).center();
    let goal = zones::TARGET.center();
    if let Some(a) = start {
        // Тип дорог и четыре числа: x — север, y — восток (оси MapPoint).
        let road: Option<Vec<MapPoint>> = ctx
            .dcs()
            .land()
            .path_on_roads("roads", a.north, a.east, goal.north, goal.east)
            .await?;
        match road {
            // Точки от начала к концу: длина пути по дорогам.
            Some(points) if points.len() > 1 => {
                let metres: f64 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
                ctx.report().set("armor_road_km", (metres / 1000.0).round());
            }
            _ => ctx.report().set("armor_road_km", "нет пути"),
        }
    }
    ctx.end_session(Outcome::Draw).await
}
// snippet:end sdk-dcs-road-path

// snippet:start sdk-dcs-road-summary
#[sortie::mission(api_rev = 3)] // land().road_path — API ревизии 3
pub async fn road_summary(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    let armor = ctx.group(groups::ARMOR)?; // Group<Ground>
    let goal = zones::TARGET.center();
    let Some(from) = ctx.world().group(&armor).center() else {
        return ctx.end_session(Outcome::Draw).await;
    };
    // Сводку считает слой Sortie внутри DCS: длина и концы пути приходят без всех его точек.
    // Some(8.0) — вернуть не больше 8 точек: оба конца и места самых крутых поворотов.
    let summary = ctx
        .dcs()
        .land()
        .road_path("roads", from, goal, Some(8.0), None)
        .await?;
    match summary {
        // Дорога подходит к цели ближе 300 м: колонна едет по дорогам через повороты.
        Some(p) if p.points.len() > 1 && p.end_gap_m.is_some_and(|gap| gap < 300.0) => {
            ctx.report()
                .set("armor_road_km", (p.length_m / 1000.0).round());
            let via_turns = p.points[1..].iter().map(|&q| RoutePoint::on_road(q));
            armor.set_task(Task::ground_route(via_turns)).await?;
        }
        _ => ctx.report().set("armor_road_km", "нет пути"),
    }
    ctx.end_session(Outcome::Draw).await
}
// snippet:end sdk-dcs-road-summary

// snippet:start sdk-dcs-farp-slots
#[sortie::mission(api_rev = 3)] // чтения экрана слотов — API ревизии 3
pub async fn farp_slots(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    let server = ctx.dcs().server();
    // FARP и корабли, которые знает экран слотов на сервере: id юнита -> {name, coalition, ..}.
    let known: Value = server.farps_and_carriers().await?;
    let farps = known
        .get("farps")
        .and_then(Value::as_table)
        .unwrap_or_default();
    for (key, entry) in farps {
        let (Some(unit_id), Some(name)) = (key.as_f64(), entry.get("name").and_then(Value::as_str))
        else {
            continue;
        };
        // false: это не аэродром, а FARP или корабль, и id — его юнита.
        let settings: Option<Value> = server.dynamic_spawn_settings(unit_id, false).await?;
        let dynamic = settings
            .as_ref()
            .and_then(|t| t.get("dynamicSpawnAvailable"))
            .and_then(Value::as_bool);
        let hueys: f64 = server.aircraft_in_warehouse("UH-1H", unit_id, true).await?;
        ctx.report().set(
            name,
            format!("динамические слоты: {dynamic:?}, UH-1H на складе: {hueys}"),
        );
    }
    ctx.end_session(Outcome::Draw).await
}
// snippet:end sdk-dcs-farp-slots

pub async fn objects(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-dcs-objects
    let dcs = ctx.dcs();
    // Объекты возвращаются как DcsObject: Eid зеркала, имя, тип.
    let units: Vec<DcsObject> = dcs.group(groups::SAM).units().await?;
    let mut damaged = Vec::new();
    for u in &units {
        let life: f64 = dcs.unit(u).life().await?;
        let life0: f64 = dcs.unit(u).life0().await?;
        if life < life0 {
            damaged.push(u.name.clone());
        }
    }
    // Унаследованные методы есть у каждого класса: Object.getPoint у юнита.
    let point: Vec3 = dcs.unit(&units[0]).point().await?;
    // Группа по имени DCS, если у неё нет символа сценария.
    let size: f64 = dcs.group(GroupSel::name("Armor")).size().await?;
    // snippet:end sdk-dcs-objects
    let _ = (point, size, damaged);
    Ok(())
}

pub async fn mutators(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-dcs-mutators
    let dcs = ctx.dcs();
    // Груз в кабине вертолёта: изменение, поэтому с ключом.
    dcs.effects()
        .set_unit_internal_cargo(UnitSel::name("Rescue 1-1"), 800.0)
        .key("rescue-cargo-1")
        .await?;
    // Склад аэродрома: по селектору аэродрома. Чтения ключа не берут.
    let stock = dcs.warehouse(airbases::KUTAISI);
    let left: f64 = stock.item_count("weapons.missiles.AIM_120C").await?;
    // Или по wsType, четырём числам из Scripts/Database/wsTypes.lua игры.
    let aim9: f64 = stock.item_count([4, 4, 7, 22]).await?;
    let inventory: Value = stock.inventory(None).await?; // большая таблица: класс heavy
    // snippet:end sdk-dcs-mutators
    let _ = (left, aim9, inventory);
    Ok(())
}

pub async fn effects_and_sounds(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-dcs-effects
    let dcs = ctx.dcs();
    let target = zones::TARGET.center().alt(m(0));
    dcs.effects()
        .explosion(target, 500.0) // мощность не больше 5000
        .key("depot-blast")
        .await?;
    dcs.effects()
        .signal_flare(target, "Red", 0.0) // ключ таблицы trigger.flareColor
        .key("flare-1")
        .await?;
    // Звук — файл внутри .miz: относительный путь, без "..", диска и ведущего "/".
    dcs.sounds()
        .out_sound_for_coalition(Coalition::Blue, "l10n/DEFAULT/alarm.ogg")
        .key("alarm-1")
        .await?;
    // snippet:end sdk-dcs-effects
    Ok(())
}

pub async fn spots(ctx: Ctx<MyMission>, jtac: DcsObject) -> sortie::Result<()> {
    // snippet:start sdk-dcs-spots
    let dcs = ctx.dcs();
    let target = zones::TARGET.center().alt(m(0));
    // Лазерное пятно живёт под дескриптором, пока его не уберут или не кончится сессия.
    let spot: Handle = dcs
        .spots()
        .create_laser(&jtac, None, target, Some(1688.0))
        .key("lase-1")
        .await?;
    dcs.spot(spot).set_point(target).key("lase-1-move").await?;
    dcs.spot(spot).destroy().key("lase-1-off").await?;
    // snippet:end sdk-dcs-spots
    Ok(())
}

pub async fn errors(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-dcs-errors
    match ctx
        .dcs()
        .sounds()
        .out_sound("l10n/DEFAULT/alarm.ogg")
        .key("alarm-all")
        .await
    {
        Ok(()) => {}
        // Владелец движка не включил функцию: capability call:mission:trigger.action.outSound.
        Err(Error::Rejected(_)) => ctx.report().set("alarm", "sounds are off"),
        Err(e) => return Err(e),
    }
    // snippet:end sdk-dcs-errors
    Ok(())
}

pub async fn limits(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-dcs-limit
    let land = ctx.dcs().land();
    let spot = zones::TARGET.center();
    let mut waits = 0;
    let surface = loop {
        match land.surface_type(spot).await {
            Ok(kind) => break kind,
            // Лимит: подождать, сколько назвал движок, и отправить то же чтение снова.
            Err(e) if waits < 3 && e.retry_after().is_some() => {
                waits += 1;
                if let Some(l) = e.limit() {
                    ctx.report().set("last_limit", l.budget.as_str());
                }
                ctx.sleep(e.retry_after().unwrap_or_default()).await;
            }
            // Запрет (функция выключена или её нет в каталоге): повтор не поможет.
            Err(e) if e.is_forbidden() => return Ok(()),
            Err(e) => return Err(e),
        }
    };
    // snippet:end sdk-dcs-limit
    let _ = surface;
    Ok(())
}

pub async fn owner_option(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-dcs-owner-enum
    use sortie::dcs::{arg, recv};
    use sortie::sel::ControllerSel;
    // Запись владельца `Controller.setOption` из engine.toml выше; вызов — по имени.
    let sam = || recv::controller(ControllerSel::group(groups::SAM));
    let alarm = arg::enum_key("AI.Option.Ground.id", "ALARM_STATE");
    // Ключ: число слой Sortie берёт из таблицы запущенной DCS, когда вызов выполняется.
    let red = arg::enum_key("AI.Option.Ground.val.ALARM_STATE", "RED");
    let dcs = ctx.dcs();
    dcs.call("Controller.setOption")
        .on(sam())
        .arg(alarm)
        .arg(red)
        .key("sam-alarm-red")
        .await?;
    // Или число: слой проверит, что оно есть в таблице (иначе Error::Invalid, в DCS ничего).
    dcs.call("Controller.setOption")
        .on(sam())
        .arg(arg::number(9.0)) // AI.Option.Ground.id.ALARM_STATE
        .arg(arg::number(1.0)) // GREEN
        .key("sam-alarm-green")
        .await?;
    // snippet:end sdk-dcs-owner-enum
    Ok(())
}

pub async fn raw(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-dcs-raw
    use sortie::dcs::{CallResult, arg};
    let dcs = ctx.dcs();
    let r = dcs
        .call("land.getHeight")
        .arg(arg::vec2(zones::TARGET.center()))
        .await?;
    let height = r.first().as_f64();
    // Пакет: вызовы выполняются в одном кадре; result_of берёт объект предыдущего вызова.
    let out = dcs
        .batch()
        .push(
            dcs.call("Airbase.getWarehouse")
                .on(arg::airbase(airbases::KUTAISI)),
        )
        .push(
            dcs.call("Warehouse.getItemCount")
                .on_result_of(0)
                .arg(arg::string("weapons.missiles.AIM_120C")),
        )
        .await?;
    if let Some(CallResult::Ok(count)) = out.results.get(1) {
        let _ = count.first().as_f64();
    }
    // snippet:end sdk-dcs-raw
    let _ = height;
    Ok(())
}
