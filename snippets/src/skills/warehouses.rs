//! Examples of the `sortie-mission-logic` skill: the stock of warehouses, its changes and the
//! names of items (references/api.md, "Warehouses").

use sortie::cmd::{Amount, Liquid, StockClass, WarehouseItem};
use sortie::prelude::*;
use sortie::sel::WarehouseSel;

use super::frontline::{Frontline, airbases};

// snippet:start skills-api-warehouses
#[sortie::mission(api_rev = 3)] // the stock query is API revision 3
pub async fn resupply_check(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // One warehouse whole: each class is Unlimited (DCS lists it empty) or Limited(name -> n).
    let stock = ctx.warehouse(airbases::MAYKOP).stock().await?;
    let limited = matches!(stock.weapons, Some(StockClass::Limited(_)));
    // Many warehouses, only one item and the fuel: getItemCount / getLiquidAmount, cheaper.
    let read = ctx
        .warehouses_stock([
            WarehouseSel::airbase(airbases::MAYKOP),
            WarehouseSel::airbase(airbases::KRYMSK),
        ])
        .items(["weapons.bombs.Mk_82"])
        .liquids()
        .await?;
    for w in &read.warehouses {
        let line = match (&w.value, w.error()) {
            (Some(s), _) => match s.count("weapons.bombs.Mk_82") {
                Some(Amount::Counted(n)) => format!("{n} Mk-82"),
                Some(Amount::Unlimited) => "unlimited".to_owned(), // a count above 2^29
                None => "no count".to_owned(),
            },
            (None, Some(e)) => e.code().to_owned(), // e.g. sortie::gone
            (None, None) => String::new(),
        };
        ctx.report().set(&w.name, line);
    }
    ctx.report().set("maykop_limited", limited);
    ctx.end_session(Outcome::Draw).await
}
// snippet:end skills-api-warehouses

// snippet:start skills-api-warehouse-op
#[sortie::mission(api_rev = 3)] // WarehouseOp is API revision 3
pub async fn convoy_arrived(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // One command, one DCS frame, all or nothing: move six bombs, add fuel, set a count.
    let out = ctx
        .warehouse_op()
        .transfer(
            airbases::KRYMSK,
            airbases::MAYKOP,
            "weapons.bombs.Mk_82",
            6.0,
        )
        .add(airbases::MAYKOP, Liquid::JetFuel, 5_000.0)
        .set(airbases::KRYMSK, [4, 4, 7, 22], 10.0) // an item by wsType
        .key("convoy-1-arrived") // every warehouse_op needs a key in dev, sets too
        .await?;
    if !out.applied {
        // Nothing changed: each change says why (NotEnough, Unlimited, Gone, ...).
        for c in out.failed() {
            ctx.report()
                .set(&c.warehouse.name, format!("{:?}: {}", c.outcome, c.message));
        }
    }
    ctx.end_session(Outcome::Draw).await
}
// snippet:end skills-api-warehouse-op

// snippet:start skills-api-item-names
#[sortie::mission(api_rev = 3)] // the names query is API revision 3
pub async fn name_the_stock(ctx: Ctx<Frontline>, _start: Start) -> sortie::Result<()> {
    // What this DCS calls each item and its wsType: read once per DCS process, then from memory.
    let names = ctx.item_names().await?;
    // The campaign's own list, checked against DCS before any stock query uses it.
    for item in ["weapons.bombs.Mk_82", "weapons.missiles.AIM_120C"] {
        let ws = names.ws_type_of(item); // None: DCS has no such name (or it is no item type)
        ctx.report().set(item, format!("{ws:?}"));
    }
    // A wsType from a mission's munitions, back to the name the warehouses use.
    let back = ctx
        .item_names()
        .items([WarehouseItem::from([4, 4, 7, 22])])
        .await?;
    let first = back.names.first().map(|n| n.name.clone());
    ctx.report().set(
        "4.4.7.22",
        format!("{first:?} unknown {}", back.unknown.len()),
    );
    ctx.end_session(Outcome::Draw).await
}
// snippet:end skills-api-item-names
