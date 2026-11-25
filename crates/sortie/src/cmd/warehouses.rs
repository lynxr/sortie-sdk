//! Warehouses (API revision 3): the stock of one or many warehouses in one query, and changes of
//! stock in one command.
//!
//! [`WarehouseOp`] (`ctx.warehouse_op()`, up to [`MAX_STOCK_CHANGES`] = 16 changes): `.set(..)`,
//! `.add(..)`, `.remove(..)` and `.transfer(..)` of an item or a liquid ([`StockItem`]), all run
//! in one DCS frame, all or nothing: every count is read and every change checked first (a
//! warehouse DCS has, a stock that is not unlimited, enough to remove or transfer), and if any
//! fails nothing is changed; a change DCS refuses while the changes run undoes the ones made
//! before it. The answer ([`StockChanges`]) says whether they were applied and, per change, its
//! [`StockOutcome`] and the counts before and after.
//!
//! ```ignore
//! let out = ctx
//!     .warehouse_op()
//!     .transfer(airbases::SENAKI, airbases::KOBULETI, "weapons.missiles.AIM_120C", 6.0)
//!     .add(airbases::KOBULETI, Liquid::JetFuel, 5_000.0)
//!     .key("convoy-7-arrived")
//!     .await?;
//! if !out.applied {
//!     for c in out.failed() { /* c.outcome, c.message */ }
//! }
//! ```
//!
//! A query (a read: nothing changes in DCS, no key needed) of one warehouse,
//! `ctx.warehouse(sel)` -> [`WarehouseHandle`] with `.stock()` -> [`WarehouseStock`], and of
//! many, `ctx.warehouses_stock(..)` -> [`WarehousesRead`] (each warehouse answered or with its
//! own error). By default the whole inventory is read (`Warehouse.getInventory`): the aircraft,
//! weapons and liquids, each a [`StockClass`] that is [`StockClass::Unlimited`] when DCS lists
//! the class as an empty table, which it is reported to do for a class the mission makes
//! unlimited (not measured, live check AWC-T25). `.items([..])` counts only those items
//! (`Warehouse.getItemCount`), `.liquids()` reads the four liquids (`getLiquidAmount`): cheaper,
//! and the inventory is then not read.
//!
//! ```ignore
//! let red = ctx.warehouse(airbases::SENAKI).stock().await?;
//! if let Some(StockClass::Limited(weapons)) = &red.weapons {
//!     let amraams = weapons.get("weapons.missiles.AIM_120C").copied().unwrap_or(0.0);
//! }
//! let fuel = ctx
//!     .warehouses_stock([airbases::SENAKI, airbases::KUTAISI])
//!     .items(["weapons.missiles.AIM_120C"])
//!     .liquids()
//!     .await?;
//! ```

use std::collections::BTreeMap;
use std::panic::Location;
use std::rc::Rc;

use sortie_proto::v1::{self, command::Body};
use sortie_types::Eid;

use super::{Cmd, CommandBody};
use crate::runtime::Session;
use crate::sel::WarehouseSel;
use crate::{Error, Failure, Result};

/// The most warehouses of one stock query.
pub const MAX_STOCK_WAREHOUSES: usize = 64;
/// The most items of one stock query.
pub const MAX_STOCK_ITEMS: usize = 64;

/// A warehouse item as DCS's `Warehouse` functions take it: by its name or by its wsType (the
/// raw wrappers, `ctx.dcs().warehouse(..).item_count(..)`, and the stock query take the same
/// type; it is also `sortie::dcs::WarehouseItem`).
///
/// ```ignore
/// let stock = ctx.dcs().warehouse(airbases::SENAKI);
/// let by_name = stock.item_count("weapons.missiles.AIM_9").await?;
/// let by_type = stock.item_count([4, 4, 7, 22]).await?; // the same missile, by wsType
/// ```
///
/// A wsType is four whole numbers (DCS's `Scripts/Database/wsTypes.lua`); the last one is
/// reported to change between DCS builds for some items.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WarehouseItem {
    /// `weapons.missiles.AIM_9`, an aircraft type, ...
    Name(String),
    /// `{4, 4, 7, 22}`.
    WsType([u32; 4]),
}

impl WarehouseItem {
    /// The wire form.
    #[must_use]
    pub fn to_proto(&self) -> v1::WarehouseItem {
        use v1::warehouse_item::Item;
        v1::WarehouseItem {
            item: Some(match self {
                WarehouseItem::Name(n) => Item::Name(n.clone()),
                WarehouseItem::WsType([a, b, c, d]) => Item::WsType(v1::WsType {
                    level1: *a,
                    level2: *b,
                    level3: *c,
                    level4: *d,
                }),
            }),
        }
    }

    /// From the wire form; `None` for an unset item.
    #[must_use]
    pub fn from_proto(p: &v1::WarehouseItem) -> Option<Self> {
        use v1::warehouse_item::Item;
        Some(match p.item.as_ref()? {
            Item::Name(n) => WarehouseItem::Name(n.clone()),
            Item::WsType(t) => WarehouseItem::WsType([t.level1, t.level2, t.level3, t.level4]),
        })
    }
}

impl From<&str> for WarehouseItem {
    fn from(s: &str) -> Self {
        WarehouseItem::Name(s.to_owned())
    }
}

impl From<String> for WarehouseItem {
    fn from(s: String) -> Self {
        WarehouseItem::Name(s)
    }
}

impl From<&String> for WarehouseItem {
    fn from(s: &String) -> Self {
        WarehouseItem::Name(s.clone())
    }
}

impl From<[u32; 4]> for WarehouseItem {
    fn from(t: [u32; 4]) -> Self {
        WarehouseItem::WsType(t)
    }
}

impl From<(u32, u32, u32, u32)> for WarehouseItem {
    fn from((a, b, c, d): (u32, u32, u32, u32)) -> Self {
        WarehouseItem::WsType([a, b, c, d])
    }
}

/// How much of an item or liquid a warehouse holds, as DCS answered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Amount {
    /// The count (liquids: the amount, kilograms assumed: DCS documents no unit).
    Counted(f64),
    /// DCS answered a number above 2^29, which it is reported to give for an unlimited stock
    /// (2^29 is the mark MOOSE uses; not measured, live check AWC-T25).
    Unlimited,
}

impl Amount {
    fn of(n: f64, unlimited: bool) -> Self {
        if unlimited {
            Amount::Unlimited
        } else {
            Amount::Counted(n)
        }
    }

    /// The count, `None` when unlimited.
    #[must_use]
    pub fn count(self) -> Option<f64> {
        match self {
            Amount::Counted(n) => Some(n),
            Amount::Unlimited => None,
        }
    }

    #[must_use]
    pub fn is_unlimited(self) -> bool {
        self == Amount::Unlimited
    }
}

/// One class of a warehouse's stock (aircraft or weapons) as `Warehouse.getInventory` listed
/// it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum StockClass {
    /// DCS listed the class as an empty table, which it is reported to do for a class the
    /// mission makes unlimited (not measured, live check AWC-T25; whether a limited class that
    /// holds nothing reads the same is not known).
    Unlimited,
    /// What DCS listed: the item as DCS names it, and its count.
    Limited(BTreeMap<String, f64>),
}

impl StockClass {
    fn from_proto(c: &v1::StockClass) -> Self {
        if c.unlimited {
            StockClass::Unlimited
        } else {
            StockClass::Limited(
                c.entries
                    .iter()
                    .map(|e| (e.name.clone(), e.count))
                    .collect(),
            )
        }
    }

    #[must_use]
    pub fn is_unlimited(&self) -> bool {
        matches!(self, StockClass::Unlimited)
    }

    /// What the class holds of `name`: `Some(Amount::Unlimited)` in an unlimited class, the
    /// count of a listed item, `None` for an item a limited class does not list.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Amount> {
        match self {
            StockClass::Unlimited => Some(Amount::Unlimited),
            StockClass::Limited(m) => m.get(name).copied().map(Amount::Counted),
        }
    }
}

/// A liquid of a warehouse (DCS numbers them 0..3).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Liquid {
    /// DCS 0.
    JetFuel,
    /// DCS 1.
    AviationGasoline,
    /// DCS 2: the water-methanol mixture.
    Mw50,
    /// DCS 3.
    Diesel,
    /// A key this release has no name for, as `unknown(<key>)`.
    Unknown(String),
}

impl Liquid {
    /// The wire form; `None` for [`Liquid::Unknown`], which no change can name.
    fn to_proto(&self) -> Option<v1::Liquid> {
        Some(match self {
            Liquid::JetFuel => v1::Liquid::JetFuel,
            Liquid::AviationGasoline => v1::Liquid::AviationGasoline,
            Liquid::Mw50 => v1::Liquid::Mw50,
            Liquid::Diesel => v1::Liquid::Diesel,
            Liquid::Unknown(_) => return None,
        })
    }

    fn from_proto(a: &v1::LiquidAmount) -> Option<Self> {
        use v1::Liquid as L;
        Some(match L::try_from(a.liquid).ok()? {
            L::JetFuel => Liquid::JetFuel,
            L::AviationGasoline => Liquid::AviationGasoline,
            L::Mw50 => Liquid::Mw50,
            L::Diesel => Liquid::Diesel,
            L::Unknown => Liquid::Unknown(a.unknown.clone()),
            L::Unspecified => return None,
        })
    }
}

/// The liquids of a warehouse.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Liquids {
    /// The whole inventory listed the liquids as an empty table (see [`StockClass::Unlimited`]).
    Unlimited,
    /// The amounts DCS gave, from the inventory or (`.liquids()`) from `getLiquidAmount` of
    /// each; those reads do not say whether the class is unlimited, only an amount above 2^29
    /// does ([`Amount::Unlimited`]).
    Amounts(BTreeMap<Liquid, Amount>),
}

impl Liquids {
    fn from_proto(l: &v1::LiquidStock) -> Self {
        if l.unlimited {
            return Liquids::Unlimited;
        }
        Liquids::Amounts(
            l.amounts
                .iter()
                .filter_map(|a| Some((Liquid::from_proto(a)?, Amount::of(a.amount, a.unlimited))))
                .collect(),
        )
    }

    /// What the warehouse holds of `liquid`: `Some(Amount::Unlimited)` when the liquids are
    /// unlimited, `None` when DCS gave no amount.
    #[must_use]
    pub fn get(&self, liquid: &Liquid) -> Option<Amount> {
        match self {
            Liquids::Unlimited => Some(Amount::Unlimited),
            Liquids::Amounts(m) => m.get(liquid).copied(),
        }
    }
}

/// One counted item (`.items([..])`): `Warehouse.getItemCount` as DCS answered it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ItemCount {
    pub item: WarehouseItem,
    /// `None` when DCS answered no number. This read alone cannot tell an unlimited class that
    /// DCS answers with a small number: read the whole inventory to see the class.
    pub amount: Option<Amount>,
}

/// The stock of one warehouse. The whole inventory fills `aircraft`, `weapons` and `liquids`;
/// `.items([..])` fills `items` and `.liquids()` `liquids`; the rest is `None` or empty.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct WarehouseStock {
    /// `getInventory().aircraft`; `None` when not read or DCS listed no such class.
    pub aircraft: Option<StockClass>,
    /// `getInventory().weapon`.
    pub weapons: Option<StockClass>,
    /// `getInventory().liquids`, or `getLiquidAmount` of each.
    pub liquids: Option<Liquids>,
    /// The counted items, in the order given.
    pub items: Vec<ItemCount>,
}

impl WarehouseStock {
    fn from_proto(w: &v1::WarehouseStock) -> Self {
        Self {
            aircraft: w.aircraft.as_ref().map(StockClass::from_proto),
            weapons: w.weapons.as_ref().map(StockClass::from_proto),
            liquids: w.liquids.as_ref().map(Liquids::from_proto),
            items: w
                .items
                .iter()
                .filter_map(|c| {
                    Some(ItemCount {
                        item: WarehouseItem::from_proto(c.item.as_ref()?)?,
                        amount: c.count.map(|n| Amount::of(n, c.unlimited)),
                    })
                })
                .collect(),
        }
    }

    /// What `.items([..])` counted of `item`.
    #[must_use]
    pub fn count(&self, item: impl Into<WarehouseItem>) -> Option<Amount> {
        let item = item.into();
        self.items
            .iter()
            .find(|c| c.item == item)
            .and_then(|c| c.amount)
    }
}

/// One warehouse of a stock query: its stock, or its own error.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct WarehouseRead {
    /// The warehouse as the engine resolved it: of an airbase or a static by the world's Eid
    /// when the world holds it alive, else as given.
    pub warehouse: Option<WarehouseSel>,
    /// The DCS name of its airbase or static, or the name given (empty for an Eid the world no
    /// longer knows).
    pub name: String,
    /// The world's Eid of that airbase or static (`None`: the world holds none alive).
    pub eid: Option<Eid>,
    /// What DCS answered; `None` when this warehouse has an error.
    pub value: Option<WarehouseStock>,
    err: Option<v1::CommandError>,
    failure: Failure,
}

impl WarehouseRead {
    /// DCS answered this warehouse.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.err.is_none()
    }

    /// This warehouse's error: `Error::Gone` (the world holds its airbase or static as dead,
    /// the Eid is stale, or DCS has no such warehouse), `Error::Lua` (a DCS read raised),
    /// `Error::Rejected` (its inventory no longer fit into the answer: read it in another
    /// query).
    #[must_use]
    pub fn error(&self) -> Option<Error> {
        self.err
            .as_ref()
            .map(|e| Error::from_command_error(ReadStocks::OP, self.failure.clone(), e))
    }
}

/// The result of a stock query: one entry per warehouse, in the order given.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct WarehousesRead {
    pub warehouses: Vec<WarehouseRead>,
}

impl WarehousesRead {
    /// How many warehouses DCS answered.
    #[must_use]
    pub fn ok_count(&self) -> usize {
        self.warehouses.iter().filter(|w| w.is_ok()).count()
    }
}

/// A stock query of many warehouses: `ctx.warehouses_stock(..)` (API revision 3). 1 to 64
/// warehouses, each at most once (by its DCS name, whatever the selector); `.items([..])` (up to
/// 64, each once) and `.liquids()` narrow it; `.await` -> [`WarehousesRead`]. The query fails
/// only when it could not run: an empty or too long list, a selector that names no warehouse
/// (an unknown manifest symbol, the Eid of a unit as a cargo), a warehouse or an item listed
/// twice, an empty item name.
#[derive(Clone, Debug)]
pub struct ReadStocks {
    warehouses: Vec<WarehouseSel>,
    items: Vec<WarehouseItem>,
    liquids: bool,
}

impl ReadStocks {
    pub(crate) fn new(warehouses: Vec<WarehouseSel>) -> Self {
        Self {
            warehouses,
            items: Vec::new(),
            liquids: false,
        }
    }
}

/// A stock query of one warehouse: `ctx.warehouse(sel).stock()` (API revision 3); `.await` ->
/// [`WarehouseStock`], or the warehouse's error (`Error::Gone` when DCS or the world has no such
/// warehouse).
#[derive(Clone, Debug)]
pub struct ReadStock {
    all: ReadStocks,
}

impl Cmd<ReadStocks> {
    /// Counts only these items (`Warehouse.getItemCount` of each; names or wsTypes), in this
    /// order; the whole inventory is then not read.
    pub fn items<I: Into<WarehouseItem>>(mut self, items: impl IntoIterator<Item = I>) -> Self {
        self.body.items = items.into_iter().map(Into::into).collect();
        self
    }

    /// Reads the four liquids (`Warehouse.getLiquidAmount` of each); the whole inventory is then
    /// not read.
    pub fn liquids(mut self) -> Self {
        self.body.liquids = true;
        self
    }
}

impl Cmd<ReadStock> {
    /// Counts only these items (see `Cmd<ReadStocks>::items`).
    pub fn items<I: Into<WarehouseItem>>(mut self, items: impl IntoIterator<Item = I>) -> Self {
        self.body.all.items = items.into_iter().map(Into::into).collect();
        self
    }

    /// Reads the four liquids (see `Cmd<ReadStocks>::liquids`).
    pub fn liquids(mut self) -> Self {
        self.body.all.liquids = true;
        self
    }
}

impl CommandBody for ReadStocks {
    type Output = WarehousesRead;
    const OP: &'static str = "warehouse_stock";
    const SINCE_API_REV: u32 = 3;
    fn target(&self) -> String {
        match self.warehouses.as_slice() {
            [one] => format!("warehouse {:?}", one.to_proto().sel),
            many => format!("{} warehouses", many.len()),
        }
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Query(v1::Query {
            kind: Some(v1::query::Kind::WarehouseStock(v1::WarehouseStocks {
                warehouses: self.warehouses.iter().map(WarehouseSel::to_proto).collect(),
                items: self.items.iter().map(WarehouseItem::to_proto).collect(),
                liquids: self.liquids,
            })),
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<WarehousesRead> {
        let read = r
            .reply
            .as_ref()
            .and_then(|x| x.warehouse_stock.clone())
            .unwrap_or_default();
        let warehouses = read
            .warehouses
            .into_iter()
            .map(|w| WarehouseRead {
                failure: Failure {
                    op: Self::OP,
                    target: format!("warehouse {:?}", w.name),
                    command_id: r.id.clone(),
                    session: s.id.clone(),
                    ..Failure::default()
                },
                warehouse: w.warehouse.clone().map(WarehouseSel::from_proto),
                name: w.name.clone(),
                eid: Eid::from_u64(w.eid),
                value: w.err.is_none().then(|| WarehouseStock::from_proto(&w)),
                err: w.err,
            })
            .collect();
        Ok(WarehousesRead { warehouses })
    }
}

impl CommandBody for ReadStock {
    type Output = WarehouseStock;
    const OP: &'static str = ReadStocks::OP;
    const SINCE_API_REV: u32 = 3;
    fn target(&self) -> String {
        self.all.target()
    }
    fn to_proto(&self, s: &Session) -> Result<Body> {
        self.all.to_proto(s)
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<WarehouseStock> {
        let target = self.all.target();
        let out = self.all.decode(s, r.clone())?;
        let Some(w) = out.warehouses.into_iter().next() else {
            return Err(Error::with_code(
                v1::ErrorCode::BridgeFault,
                Failure {
                    op: Self::OP,
                    target,
                    message: "the engine answered no warehouse".into(),
                    command_id: r.id,
                    session: s.id.clone(),
                    ..Failure::default()
                },
            ));
        };
        if let Some(e) = w.error() {
            return Err(e);
        }
        w.value.ok_or_else(|| {
            Error::with_code(
                v1::ErrorCode::BridgeFault,
                Failure {
                    op: Self::OP,
                    target,
                    message: "the engine answered the warehouse without its stock".into(),
                    ..Failure::default()
                },
            )
        })
    }
}

/// One warehouse to read (`ctx.warehouse(sel)`): `.stock()`, a query of API revision 3
/// (declare `#[sortie::mission(api_rev = 3)]`).
#[derive(Clone, Debug)]
pub struct WarehouseHandle {
    session: Rc<Session>,
    warehouse: WarehouseSel,
}

impl WarehouseHandle {
    pub(crate) fn new(session: Rc<Session>, warehouse: WarehouseSel) -> Self {
        Self { session, warehouse }
    }

    /// Its stock: the whole inventory (`Warehouse.getInventory`) unless `.items([..])` or
    /// `.liquids()` narrow it.
    #[track_caller]
    pub fn stock(&self) -> Cmd<ReadStock> {
        Cmd::new(
            self.session.clone(),
            ReadStock {
                all: ReadStocks::new(vec![self.warehouse.clone()]),
            },
            Location::caller(),
        )
    }
}

// ----- the change of stock ----------------------------------------------------------------------

/// The most changes of one [`WarehouseOp`]: all of them run in one DCS frame, about 3 DCS calls
/// a set, add or remove and 6 a transfer, so a longer list goes in several commands (each its
/// own frame, each all or nothing on its own). What a change costs is not measured yet (live
/// check AWC-T26).
pub const MAX_STOCK_CHANGES: usize = 16;

/// What a change of stock changes: an item (by its name or wsType, as [`WarehouseItem`]) or a
/// liquid. `"weapons.missiles.AIM_120C"`, `[4, 4, 7, 106]` and `Liquid::JetFuel` all convert.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StockItem {
    /// `Warehouse.setItem`, `addItem`, `removeItem`: a whole count.
    Item(WarehouseItem),
    /// `Warehouse.setLiquidAmount`, `addLiquid`, `removeLiquid`: an amount (kilograms assumed:
    /// DCS documents no unit).
    Liquid(Liquid),
}

impl From<WarehouseItem> for StockItem {
    fn from(i: WarehouseItem) -> Self {
        StockItem::Item(i)
    }
}

impl From<Liquid> for StockItem {
    fn from(l: Liquid) -> Self {
        StockItem::Liquid(l)
    }
}

impl From<&str> for StockItem {
    fn from(s: &str) -> Self {
        StockItem::Item(s.into())
    }
}

impl From<String> for StockItem {
    fn from(s: String) -> Self {
        StockItem::Item(s.into())
    }
}

impl From<&String> for StockItem {
    fn from(s: &String) -> Self {
        StockItem::Item(s.into())
    }
}

impl From<[u32; 4]> for StockItem {
    fn from(t: [u32; 4]) -> Self {
        StockItem::Item(t.into())
    }
}

impl From<(u32, u32, u32, u32)> for StockItem {
    fn from(t: (u32, u32, u32, u32)) -> Self {
        StockItem::Item(t.into())
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Change {
    Set(f64),
    Add(f64),
    Remove(f64),
    Transfer(WarehouseSel, f64),
}

#[derive(Clone, Debug, PartialEq)]
struct Planned {
    warehouse: WarehouseSel,
    what: StockItem,
    change: Change,
}

/// Changes of warehouse stock in one DCS frame, all or nothing: `ctx.warehouse_op()` (API
/// revision 3), then `.set(..)`, `.add(..)`, `.remove(..)`, `.transfer(..)` (1 to
/// [`MAX_STOCK_CHANGES`], applied in the order given), `.key(..)`, `.await` ->
/// [`StockChanges`]. An item is counted in whole numbers up to 1 000 000, a liquid up to
/// 100 000 000; `set` takes 0 too, the others more than 0. The command fails only when it could
/// not run: no change or too many, a warehouse the world does not hold alive (`Error::Invalid`;
/// `Error::Gone` for a dead one), an amount out of range, a transfer to its own warehouse,
/// [`Liquid::Unknown`]. In dev mode (and on an engine that requires keys) every `warehouse_op`
/// needs `.key(..)`: a reloaded logic runs it again, and unkeyed an add, remove or transfer
/// would move the stock twice and a set would put a count back over what was used since.
#[derive(Clone, Debug, Default)]
pub struct WarehouseOp {
    changes: Vec<Planned>,
}

impl WarehouseOp {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, warehouse: WarehouseSel, what: StockItem, change: Change) {
        self.changes.push(Planned {
            warehouse,
            what,
            change,
        });
    }
}

impl Cmd<WarehouseOp> {
    /// Sets the count (an item) or amount (a liquid) of `what` in `warehouse` to `amount`.
    pub fn set(
        mut self,
        warehouse: impl Into<WarehouseSel>,
        what: impl Into<StockItem>,
        amount: f64,
    ) -> Self {
        self.body
            .push(warehouse.into(), what.into(), Change::Set(amount));
        self
    }

    /// Adds `amount` of `what` to `warehouse`.
    pub fn add(
        mut self,
        warehouse: impl Into<WarehouseSel>,
        what: impl Into<StockItem>,
        amount: f64,
    ) -> Self {
        self.body
            .push(warehouse.into(), what.into(), Change::Add(amount));
        self
    }

    /// Removes `amount` of `what` from `warehouse`: more than it holds (after the changes
    /// before this one) is refused ([`StockOutcome::NotEnough`]), never clamped at zero.
    pub fn remove(
        mut self,
        warehouse: impl Into<WarehouseSel>,
        what: impl Into<StockItem>,
        amount: f64,
    ) -> Self {
        self.body
            .push(warehouse.into(), what.into(), Change::Remove(amount));
        self
    }

    /// Moves `amount` of `what` from `from` to `to` (another warehouse): removed there and
    /// added here in the same frame, or not at all.
    pub fn transfer(
        mut self,
        from: impl Into<WarehouseSel>,
        to: impl Into<WarehouseSel>,
        what: impl Into<StockItem>,
        amount: f64,
    ) -> Self {
        self.body.push(
            from.into(),
            what.into(),
            Change::Transfer(to.into(), amount),
        );
        self
    }
}

/// What a [`WarehouseOp`] did with one change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StockOutcome {
    /// Applied: `after` is what DCS read back.
    Done,
    /// This change passed its check, but another one did not: nothing was changed.
    NotApplied,
    /// Applied and undone again, because a later change failed while the changes ran.
    Undone,
    /// Applied, and the undo after a later failure did not bring its count back: the stock
    /// stays as `after` says ([`StockChanges::undo_failed`]).
    UndoFailed,
    /// The stock is unlimited (DCS answered a count above 2^29, on either side of a transfer).
    /// DCS is reported to change nothing on an unlimited stock (not measured, live check
    /// AWC-T26), so the change is refused before anything is changed.
    Unlimited,
    /// A remove or a transfer of more than the warehouse holds.
    NotEnough,
    /// DCS has no such warehouse now.
    Gone,
    /// A DCS read or change raised (`message` names the function).
    DcsError,
    /// DCS did not change the count as asked (as an unlimited stock that DCS answers with a
    /// small count would do; not measured, AWC-T26): the changes before it were undone.
    Unchanged,
    /// When its turn came the count was not what the check left for it: an earlier change of
    /// the command moved it under another form of the item (its name and its wsType, or two
    /// names DCS counts as one stock), which the check counted apart. It did not run; the
    /// changes before it were undone. Name an item in one form per command
    /// ([`ctx.item_names()`](crate::Ctx::item_names) goes between the two).
    Aliased,
    /// An outcome this release does not know.
    Unknown,
}

impl StockOutcome {
    fn from_proto(o: i32) -> Self {
        use v1::StockOutcome as O;
        match O::try_from(o).unwrap_or(O::Unspecified) {
            O::Done => StockOutcome::Done,
            O::NotApplied => StockOutcome::NotApplied,
            O::Undone => StockOutcome::Undone,
            O::UndoFailed => StockOutcome::UndoFailed,
            O::Unlimited => StockOutcome::Unlimited,
            O::NotEnough => StockOutcome::NotEnough,
            O::Gone => StockOutcome::Gone,
            O::DcsError => StockOutcome::DcsError,
            O::Unchanged => StockOutcome::Unchanged,
            O::Aliased => StockOutcome::Aliased,
            O::Unspecified => StockOutcome::Unknown,
        }
    }

    /// The reason a command was not applied: not `Done`, `NotApplied` or `Undone`.
    #[must_use]
    pub fn is_failure(self) -> bool {
        !matches!(
            self,
            StockOutcome::Done | StockOutcome::NotApplied | StockOutcome::Undone
        )
    }
}

/// One warehouse of a change, and its counts of the item or liquid.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct StockSide {
    /// The warehouse as the engine resolved it: of an airbase or a static by the world's Eid
    /// when the world holds it alive, else as given.
    pub warehouse: Option<WarehouseSel>,
    /// The DCS name of its airbase or static, or the name given.
    pub name: String,
    /// The world's Eid of that airbase or static.
    pub eid: Option<Eid>,
    /// The count this change starts from: what DCS read before the command, moved by the
    /// changes before this one; `None` when the warehouse was not found or the read failed.
    pub before: Option<f64>,
    /// What DCS read after this change (after the undo for `Undone`, `UndoFailed` and the
    /// change that failed while the changes ran); `None` when it did not run.
    pub after: Option<f64>,
    /// `before` is above 2^29: an unlimited stock, as DCS is reported to answer one.
    pub unlimited: bool,
}

impl StockSide {
    fn from_proto(s: &v1::StockSide) -> Self {
        Self {
            warehouse: s.warehouse.clone().map(WarehouseSel::from_proto),
            name: s.name.clone(),
            eid: Eid::from_u64(s.eid),
            before: s.before,
            after: s.after,
            unlimited: s.unlimited,
        }
    }
}

/// What a [`WarehouseOp`] did with one change, in the order given.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StockChanged {
    /// The item or liquid of the change, as given.
    pub what: StockItem,
    pub outcome: StockOutcome,
    /// Why, for every outcome but `Done`.
    pub message: String,
    /// The warehouse changed; for a transfer, the one the stock left.
    pub warehouse: StockSide,
    /// A transfer: the warehouse the stock went to.
    pub to: Option<StockSide>,
}

/// The answer of a [`WarehouseOp`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StockChanges {
    /// Every change was applied. `false`: none is (each change's outcome says why), unless
    /// `undo_failed`.
    pub applied: bool,
    /// A change failed while the changes ran and the undo did not bring every count back: the
    /// stock is as the `after` of each side says.
    pub undo_failed: bool,
    /// One per change, in the order given.
    pub changes: Vec<StockChanged>,
}

impl StockChanges {
    /// The changes that kept the command from being applied ([`StockOutcome::is_failure`]).
    pub fn failed(&self) -> impl Iterator<Item = &StockChanged> {
        self.changes.iter().filter(|c| c.outcome.is_failure())
    }
}

impl CommandBody for WarehouseOp {
    type Output = StockChanges;
    const OP: &'static str = "warehouse_op";
    const SINCE_API_REV: u32 = 3;
    const KEY_REQUIRED_IN_DEV: bool = true;
    fn target(&self) -> String {
        match self.changes.as_slice() {
            [one] => format!("warehouse {:?}", one.warehouse.to_proto().sel),
            many => format!("{} changes of stock", many.len()),
        }
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        use v1::stock_change::{Change as C, Stock};
        let mut changes = Vec::with_capacity(self.changes.len());
        for (i, c) in self.changes.iter().enumerate() {
            let stock = match &c.what {
                StockItem::Item(item) => Stock::Item(item.to_proto()),
                StockItem::Liquid(l) => match l.to_proto() {
                    Some(l) => Stock::Liquid(l as i32),
                    None => {
                        return Err(Error::Invalid(Box::new(Failure::new(
                            Self::OP,
                            format!("change {}: {l:?} is no liquid a change can name", i + 1),
                        ))));
                    }
                },
            };
            let change = match &c.change {
                Change::Set(n) => C::Set(*n),
                Change::Add(n) => C::Add(*n),
                Change::Remove(n) => C::Remove(*n),
                Change::Transfer(to, n) => C::Transfer(v1::StockTransfer {
                    to: Some(to.to_proto()),
                    amount: *n,
                }),
            };
            changes.push(v1::StockChange {
                warehouse: Some(c.warehouse.to_proto()),
                stock: Some(stock),
                change: Some(change),
            });
        }
        Ok(Body::WarehouseOp(v1::WarehouseOp { changes }))
    }
    fn decode(self, _: &Rc<Session>, r: v1::CommandResult) -> Result<StockChanges> {
        let got = r.reply.and_then(|x| x.warehouse_op).unwrap_or_default();
        let changes = got
            .changes
            .iter()
            .zip(self.changes)
            .map(|(c, asked)| StockChanged {
                what: asked.what,
                outcome: StockOutcome::from_proto(c.outcome),
                message: c.message.clone(),
                warehouse: c
                    .warehouse
                    .as_ref()
                    .map(StockSide::from_proto)
                    .unwrap_or_default(),
                to: c.to.as_ref().map(StockSide::from_proto),
            })
            .collect();
        Ok(StockChanges {
            applied: got.applied,
            undo_failed: got.undo_failed,
            changes,
        })
    }
}
