//! The names of items (API revision 3): what the running DCS calls each object it knows, with its
//! wsType, so a logic names a weapon for a warehouse without guessing DCS's strings and goes
//! between a weapon's name and its wsType.
//!
//! `ctx.item_names()` -> [`ItemNames`] asks the DCS the engine is linked to (its
//! `Warehouse.getResourceMap()`, documented since DCS 2.8.8; not measured, live check AWC-T27):
//! every name with its wsType, sorted by name, weapons as `weapons.missiles.AIM_120C`, aircraft
//! by their type. `.items([..])` looks up only those (a name, or a wsType, which may belong to
//! several names). The engine reads DCS's map once per DCS process and version and answers later
//! queries from memory ([`ItemNames::cached`]); nothing of it is written anywhere, and nothing
//! of DCS's data ships with the SDK. A query is a read: nothing changes in DCS, and it takes no
//! key (with `.key(..)` the engine answers `Error::Invalid`: the map is never stored).
//!
//! ```ignore
//! let names = ctx.item_names().await?;
//! let amraam = names.ws_type_of("weapons.missiles.AIM_120C"); // Some([4, 4, 7, 106])
//! let same: Vec<&str> = names.names_of([4, 4, 7, 106]).collect();
//! let looked_up = ctx
//!     .item_names()
//!     .items(["weapons.missiles.AIM_9M", "weapons.missiles.NO_SUCH"])
//!     .await?;
//! assert_eq!(looked_up.unknown.len(), 1);
//! ```

use std::rc::Rc;

use sortie_proto::v1::{self, command::Body};

use super::{Cmd, CommandBody, WarehouseItem};
use crate::Result;
use crate::runtime::Session;

/// The most items one names query looks up.
pub const MAX_NAME_LOOKUPS: usize = 64;

/// One object as DCS names it, with its wsType.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ItemName {
    /// As DCS names it: `weapons.missiles.AIM_120C`, an aircraft type such as `F-16C_50`.
    pub name: String,
    /// Its wsType; `None` where DCS gives `{0, 0, 0, 0}`, which it gives shells, statics and
    /// turrets alike (no item type).
    pub ws_type: Option<[u32; 4]>,
}

impl ItemName {
    /// The item by its name, as the warehouse queries and `ctx.warehouse_op()` take it.
    #[must_use]
    pub fn item(&self) -> WarehouseItem {
        WarehouseItem::Name(self.name.clone())
    }
}

/// The answer of a names query.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ItemNames {
    /// The names answered, sorted by name, each once: every name DCS knows, or (`.items([..])`)
    /// those the lookups found.
    pub names: Vec<ItemName>,
    /// The items looked up that DCS has no entry for, in the order given.
    pub unknown: Vec<WarehouseItem>,
    /// How many names DCS's map holds.
    pub total: usize,
    /// Entries of DCS's map the layer left out (no name, or not four whole numbers).
    pub skipped: usize,
    /// The DCS version that answered.
    pub dcs_version: String,
    /// Answered from what the engine kept of an earlier read in the same DCS process.
    pub cached: bool,
    /// How the layer called DCS (`Warehouse.getResourceMap()`, or with a warehouse).
    pub source: String,
}

impl ItemNames {
    fn from_proto(l: v1::ItemNameList) -> Self {
        Self {
            names: l
                .names
                .into_iter()
                .map(|n| ItemName {
                    name: n.name,
                    ws_type: n.ws_type.map(|w| [w.level1, w.level2, w.level3, w.level4]),
                })
                .collect(),
            unknown: l
                .unknown
                .iter()
                .filter_map(WarehouseItem::from_proto)
                .collect(),
            total: l.total as usize,
            skipped: l.skipped as usize,
            dcs_version: l.dcs_version,
            cached: l.cached,
            source: l.source,
        }
    }

    /// The entry of `name`, if DCS answered it.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ItemName> {
        self.names
            .binary_search_by(|n| n.name.as_str().cmp(name))
            .ok()
            .map(|i| &self.names[i])
    }

    /// The wsType of `name` (`None`: not answered, or no item type).
    #[must_use]
    pub fn ws_type_of(&self, name: &str) -> Option<[u32; 4]> {
        self.get(name).and_then(|n| n.ws_type)
    }

    /// Every name answered with the wsType `ws`, in name order (usually one).
    pub fn names_of(&self, ws: [u32; 4]) -> impl Iterator<Item = &str> {
        self.names
            .iter()
            .filter(move |n| n.ws_type == Some(ws))
            .map(|n| n.name.as_str())
    }

    /// Whether DCS answered `item`: a name it has, or a wsType one of its names has.
    #[must_use]
    pub fn knows(&self, item: impl Into<WarehouseItem>) -> bool {
        match item.into() {
            WarehouseItem::Name(n) => self.get(&n).is_some(),
            WarehouseItem::WsType(w) => self.names_of(w).next().is_some(),
        }
    }
}

/// A names query: `ctx.item_names()` (API revision 3); `.items([..])` (up to
/// [`MAX_NAME_LOOKUPS`], each once) narrows it; `.await` -> [`ItemNames`]. It fails only when it
/// could not run: too many or invalid lookups, `Error::Unavailable` for a DCS without
/// `Warehouse.getResourceMap` (before 2.8.8).
#[derive(Clone, Debug, Default)]
pub struct ReadItemNames {
    items: Vec<WarehouseItem>,
}

impl Cmd<ReadItemNames> {
    /// Looks up only these items (names or wsTypes), each at most once.
    pub fn items<I: Into<WarehouseItem>>(mut self, items: impl IntoIterator<Item = I>) -> Self {
        self.body.items = items.into_iter().map(Into::into).collect();
        self
    }
}

impl CommandBody for ReadItemNames {
    type Output = ItemNames;
    const OP: &'static str = "item_names";
    const SINCE_API_REV: u32 = 3;
    fn target(&self) -> String {
        match self.items.len() {
            0 => "every item".into(),
            n => format!("{n} items"),
        }
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Query(v1::Query {
            kind: Some(v1::query::Kind::ItemNames(v1::ItemNames {
                items: self.items.iter().map(WarehouseItem::to_proto).collect(),
            })),
        }))
    }
    fn decode(self, _: &Rc<Session>, r: v1::CommandResult) -> Result<ItemNames> {
        Ok(ItemNames::from_proto(
            r.reply.and_then(|x| x.item_names).unwrap_or_default(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> ItemNames {
        ItemNames::from_proto(v1::ItemNameList {
            names: [
                ("A-plane", Some([1, 1, 6, 58])),
                ("m.one", Some([4, 4, 7, 1])),
                ("m.two", Some([4, 4, 7, 1])),
                ("s", None),
            ]
            .into_iter()
            .map(|(n, w)| v1::ItemName {
                name: n.into(),
                ws_type: w.map(|w: [u32; 4]| v1::WsType {
                    level1: w[0],
                    level2: w[1],
                    level3: w[2],
                    level4: w[3],
                }),
            })
            .collect(),
            total: 4,
            ..v1::ItemNameList::default()
        })
    }

    #[test]
    fn lookups_go_between_a_name_and_its_ws_type() {
        let l = list();
        assert_eq!(l.ws_type_of("m.one"), Some([4, 4, 7, 1]));
        assert_eq!(l.ws_type_of("s"), None, "no item type");
        assert_eq!(l.ws_type_of("nothing"), None);
        assert_eq!(
            l.names_of([4, 4, 7, 1]).collect::<Vec<_>>(),
            ["m.one", "m.two"]
        );
        assert!(l.knows("A-plane") && l.knows([1, 1, 6, 58]));
        assert!(!l.knows("nothing") && !l.knows([0, 0, 0, 0]));
        assert_eq!(
            l.get("m.two").map(ItemName::item),
            Some(WarehouseItem::from("m.two"))
        );
        assert_eq!(l.total, 4);
    }
}
