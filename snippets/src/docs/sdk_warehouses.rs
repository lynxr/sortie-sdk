//! Warehouses (API revision 3): the stock of one or many warehouses, the whole inventory or
//! only some items and the liquids; changes of stock in one command (`ctx.warehouse_op()`); the
//! names the running DCS gives items, with their wsTypes (`ctx.item_names()`).

use sortie::cmd::WarehouseItem;
use sortie::cmd::{Amount, Liquid, Liquids, StockClass, StockOutcome};
use sortie::prelude::*;
use sortie::sel::{AirbaseSel, WarehouseSel};

use super::my_mission::{MyMission, airbases};

// snippet:start sdk-warehouses-stock
#[sortie::mission(api_rev = 3)] // ctx.warehouse(..).stock() — API ревизии 3
pub async fn count_missiles(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    // Весь склад Нальчика: самолёты, оружие и топливо, каждый класс — безлимитный или список.
    let stock = ctx.warehouse(airbases::NALCHIK).stock().await?;
    match &stock.weapons {
        // Пустой класс DCS считается безлимитным: так DCS, по сообщениям, отдаёт безлимитный.
        Some(StockClass::Unlimited) => ctx.report().set("weapons", "без лимита"),
        // Ограниченный: имя предмета, как его называет DCS, и сколько его.
        Some(StockClass::Limited(items)) => {
            let amraam = items
                .get("weapons.missiles.AIM_120C")
                .copied()
                .unwrap_or(0.0);
            ctx.report().set("amraam", amraam);
        }
        _ => {}
    }
    // Топливо — по имени жидкости (килограммы, но единицу DCS не документирует).
    if let Some(Liquids::Amounts(fuel)) = &stock.liquids {
        let jet = fuel.get(&Liquid::JetFuel).and_then(|a| a.count());
        ctx.report().set("jet_fuel", format!("{jet:?}"));
    }
    ctx.end_session(Outcome::Draw).await
}
// snippet:end sdk-warehouses-stock

pub async fn read_items(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-warehouses-items
    // Только нужные предметы — getItemCount каждого, дешевле всего склада. Предмет — имя или
    // wsType из четырёх чисел; ответ — в том же порядке.
    let s = ctx
        .warehouse(airbases::NALCHIK)
        .stock()
        .items([
            WarehouseItem::from("weapons.missiles.AIM_120C"),
            WarehouseItem::from([4, 4, 7, 22]), // AIM-9M по wsType
        ])
        .liquids() // и четыре жидкости, getLiquidAmount каждой
        .await?;
    match s.count("weapons.missiles.AIM_120C") {
        Some(Amount::Counted(n)) => ctx.report().set("amraam", n),
        // Число выше 2^29: DCS, по сообщениям, так отвечает про безлимитный склад.
        Some(Amount::Unlimited) => ctx.report().set("amraam", "без лимита"),
        None => {}
    }
    // snippet:end sdk-warehouses-items
    Ok(())
}

pub async fn read_many(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-warehouses-batch
    // До 64 складов одним запросом: аэродрома, груза (статика) или по имени склада в DCS.
    let read = ctx
        .warehouses_stock([
            WarehouseSel::airbase(airbases::NALCHIK),
            WarehouseSel::airbase(AirbaseSel::name("FARP North")),
            WarehouseSel::named("Kutaisi"),
        ])
        .items(["weapons.missiles.AIM_120C"])
        .await?;
    for w in &read.warehouses {
        match (&w.value, w.error()) {
            (Some(s), _) => {
                let n = s.count("weapons.missiles.AIM_120C");
                ctx.report().set(&w.name, format!("{n:?}"));
            }
            // Своя ошибка склада: Error::Gone — его нет в DCS или мир держит базу мёртвой.
            (None, Some(e)) => ctx.report().set(&w.name, e.code()),
            (None, None) => {}
        }
    }
    ctx.report().set("answered", read.ok_count());
    // snippet:end sdk-warehouses-batch
    Ok(())
}

pub async fn change_stock(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-warehouses-op
    // Колонна дошла: ракеты из Кутаиси в Нальчик, топливо Нальчику, две AIM-9M списаны —
    // одной командой, в одном кадре DCS, всё или ничего.
    let out = ctx
        .warehouse_op()
        .transfer(
            airbases::KUTAISI,
            airbases::NALCHIK,
            "weapons.missiles.AIM_120C",
            6.0,
        )
        .add(airbases::NALCHIK, Liquid::JetFuel, 5_000.0)
        .remove(airbases::KUTAISI, [4, 4, 7, 22], 2.0) // AIM-9M по wsType
        .key("convoy-3-arrived") // каждая команда склада — с ключом
        .await?;
    if out.applied {
        // У каждой замены — счёт до неё и что DCS прочла после; у перевода — и у склада `to`.
        let nalchik = out.changes[0].to.as_ref().and_then(|t| t.after);
        ctx.report().set("nalchik_amraam", format!("{nalchik:?}"));
    } else {
        // Ничего не изменилось. Почему — у замен, что не прошли проверку.
        for c in out.failed() {
            let why = match c.outcome {
                StockOutcome::NotEnough => "мало на складе",
                StockOutcome::Unlimited => "склад без лимита",
                StockOutcome::Gone => "склада нет в DCS",
                StockOutcome::Aliased => "один предмет назван именем и wsType",
                _ => "DCS не приняла замену",
            };
            ctx.report()
                .set(&c.warehouse.name, format!("{why}: {}", c.message));
        }
    }
    // snippet:end sdk-warehouses-op
    Ok(())
}

pub async fn set_stock(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-warehouses-set
    // Пополнение в начале миссии — тоже с ключом: без него перезагрузка логики поставила бы
    // эти числа снова, поверх того, что успели израсходовать.
    let out = ctx
        .warehouse_op()
        .set(airbases::NALCHIK, "weapons.missiles.AIM_120C", 20.0)
        .set(airbases::NALCHIK, Liquid::Diesel, 0.0)
        .key("restock-nalchik")
        .await?;
    ctx.report().set("restocked", out.applied);
    // snippet:end sdk-warehouses-set
    Ok(())
}

pub async fn weapon_names(ctx: Ctx<MyMission>) -> sortie::Result<()> {
    // snippet:start sdk-warehouses-names
    // Как эта DCS называет то, что знает, и wsType каждого — один раз за процесс DCS,
    // дальше движок отвечает из памяти.
    let names = ctx.item_names().await?;
    // Имя -> wsType и обратно.
    if let Some(ws) = names.ws_type_of("weapons.missiles.AIM_120C") {
        let same: Vec<&str> = names.names_of(ws).collect();
        ctx.report().set("amraam_ws", format!("{ws:?} {same:?}"));
    }
    // Свой список предметов кампании проверить до первого запроса склада.
    let mine = ["weapons.missiles.AIM_120C", "weapons.bombs.Mk_82"];
    let unknown: Vec<&str> = mine.into_iter().filter(|n| !names.knows(*n)).collect();
    ctx.report().set("unknown_items", format!("{unknown:?}"));
    // Только нужные: имя или wsType; чего DCS не знает — в `unknown`.
    let some = ctx
        .item_names()
        .items([WarehouseItem::from([4, 4, 7, 22])])
        .await?;
    for n in &some.names {
        ctx.report().set(&n.name, format!("{:?}", n.ws_type));
    }
    // snippet:end sdk-warehouses-names
    Ok(())
}
