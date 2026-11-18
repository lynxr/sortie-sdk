//! Objects (API revision 1): static objects, destroying any object, removing junk.
//!
//! * [`SpawnStatic`] (`ctx.spawn_static(type, country)`): a static object, a cargo or a FARP.
//!   The name is its identity; the engine knows the DCS category and model of every type of its
//!   table and refuses a wrong pair (a wrong pair is invisible in DCS). The static is in
//!   `ctx.world()` when the command returns, and a `Birth` event announces it (DCS raises none
//!   for statics).
//! * [`DestroyObject`] (`ctx.destroy(obj)`): a unit, a static or a cargo. A unit a player
//!   occupies is refused unless `.allow_player()`. A static DCS did not remove (reported for
//!   FARPs, not measured) is `Error::Lua` ("DCS did not remove .."): the world keeps it alive.
//! * [`RemoveJunk`] (`ctx.remove_junk(volumes)`): wrecks, craters and debris inside DCS
//!   volumes.
//! * [`SearchObjects`] (`ctx.search_objects(volume)`, API revision 2): the units, statics,
//!   map objects (scenery) and others DCS finds inside a volume, each with its state as DCS
//!   answered when the search ran.

use std::rc::Rc;

use sortie_proto::LuaValue;
use sortie_proto::v1::{self, command::Body};
use sortie_types::{Coalition, Eid, MapPoint, Vec3};

use super::{Cmd, CommandBody, unit};
use crate::runtime::Session;
use crate::sel::{Obj, UnitSel, Volume};
use crate::{Error, Failure, Result};

/// What a spawn does when a live object already holds the name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnConflict {
    /// `Error::NameConflict` (the default).
    #[default]
    Fail,
    /// Destroy the holder (a static that no player occupies), then spawn. The holder's
    /// `UnitGone` says `Replaced`. Any other holder is `Error::NameConflict`. A holder DCS
    /// did not remove (reported for FARPs, not measured) is `Error::Lua` ("DCS did not remove
    /// .."): nothing is spawned and the holder stays alive in the world.
    Replace,
    /// Spawn nothing: the result is the holder when it is a static of the same type
    /// ([`SpawnedStatic::kept`]); anything else is `Error::NameConflict`.
    Keep,
}

impl OnConflict {
    fn to_proto(self) -> v1::NameConflict {
        match self {
            OnConflict::Fail => v1::NameConflict::Fail,
            OnConflict::Replace => v1::NameConflict::Replace,
            OnConflict::Keep => v1::NameConflict::Keep,
        }
    }
}

/// The FARP part of a [`SpawnStatic`] of a heliport type ("FARP", "SINGLE_HELIPAD",
/// "Invisible FARP"): it makes a working airbase.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Heliport {
    callsign: String,
    frequency_hz: f64,
    fm: bool,
}

impl Heliport {
    /// Callsign chosen by the engine, 127.5 MHz AM.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The FARP callsign by name: "London", "Dallas", "Paris", "Moscow", "Berlin", "Rome",
    /// "Madrid", "Warsaw", "Dublin" or "Perth".
    #[must_use]
    pub fn callsign(mut self, name: impl Into<String>) -> Self {
        self.callsign = name.into();
        self
    }

    /// The radio frequency in hertz (127.5 MHz = `127_500_000.0`).
    #[must_use]
    pub fn frequency_hz(mut self, hz: f64) -> Self {
        self.frequency_hz = hz;
        self
    }

    /// FM instead of AM.
    #[must_use]
    pub fn fm(mut self) -> Self {
        self.fm = true;
        self
    }

    fn to_proto(&self) -> v1::HeliportSpec {
        v1::HeliportSpec {
            callsign: self.callsign.clone(),
            frequency_hz: self.frequency_hz,
            modulation: if self.fm {
                v1::Modulation::Fm
            } else {
                v1::Modulation::Am
            } as i32,
        }
    }
}

/// A spawned (or kept) static object.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SpawnedStatic {
    pub eid: Eid,
    pub name: String,
    /// The airbase a heliport became (also of a kept heliport).
    pub airbase: Option<Eid>,
    /// [`OnConflict::Keep`] found the holder: nothing was spawned.
    pub kept: bool,
}

/// `ctx.spawn_static(type, country)`; place it with `.at(..)` or `.on_unit(..)`.
#[derive(Clone, Debug)]
pub struct SpawnStatic {
    spec: v1::SpawnStatic,
}

impl SpawnStatic {
    pub(crate) fn new(type_name: String, country: String) -> Self {
        Self {
            spec: v1::SpawnStatic {
                type_name,
                country,
                ..v1::SpawnStatic::default()
            },
        }
    }
}

impl Cmd<SpawnStatic> {
    /// Explicit name (default `<type>#<session4>-<n>`); unique among units and statics.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.body.spec.name = Some(name.into());
        self
    }

    /// On the map at `position` (on the ground), facing `heading_rad`.
    pub fn at(mut self, position: MapPoint, heading_rad: f64) -> Self {
        self.body.spec.placement = Some(v1::spawn_static::Placement::At(v1::StaticAt {
            position: Some(position.into()),
            heading_rad,
        }));
        self
    }

    /// On a ship: `forward_m` / `right_m` from its centre, turned `angle_rad` from its heading.
    /// The static moves with the ship.
    pub fn on_unit(
        mut self,
        ship: impl Into<UnitSel>,
        forward_m: f64,
        right_m: f64,
        angle_rad: f64,
    ) -> Self {
        self.body.spec.placement = Some(v1::spawn_static::Placement::OnUnit(v1::OnUnit {
            unit: Some(ship.into().to_proto()),
            forward_m,
            right_m,
            angle_rad,
        }));
        self
    }

    /// The DCS category ("Fortifications", "Cargos", ...). Default: the engine's table for the
    /// type; a set category wins over it. A type the table lacks (a mod) needs it and
    /// [`Self::shape`].
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.body.spec.dcs_category = category.into();
        self
    }

    /// The model (`shape_name`); default: the table's model for the type.
    pub fn shape(mut self, shape_name: impl Into<String>) -> Self {
        self.body.spec.shape_name = shape_name.into();
        self
    }

    /// Spawn it as a wreck.
    pub fn dead(mut self) -> Self {
        self.body.spec.dead = true;
        self
    }

    /// A cargo of `mass_kg`; `liftable`: helicopters can hook it.
    pub fn cargo(mut self, mass_kg: f64, liftable: bool) -> Self {
        self.body.spec.cargo = Some(v1::CargoSpec { mass_kg, liftable });
        self
    }

    /// A FARP or helipad that works as an airbase (needed for a heliport type).
    pub fn heliport(mut self, heliport: Heliport) -> Self {
        self.body.spec.heliport = Some(heliport.to_proto());
        self
    }

    pub fn livery(mut self, livery: impl Into<String>) -> Self {
        self.body.spec.livery = livery.into();
        self
    }

    /// Score for destroying it.
    pub fn score(mut self, score: u32) -> Self {
        self.body.spec.score = score;
        self
    }

    /// Hidden on the F10 map.
    pub fn hidden(mut self) -> Self {
        self.body.spec.hidden = true;
        self
    }

    pub fn on_conflict(mut self, policy: OnConflict) -> Self {
        self.body.spec.on_conflict = policy.to_proto() as i32;
        self
    }

    /// Extra keys of the DCS static table (plain data); a key a builder method sets is
    /// refused.
    pub fn raw(mut self, table: LuaValue) -> Self {
        self.body.spec.raw = Some(table);
        self
    }
}

fn no_eid(op: &'static str) -> Error {
    Error::Invalid(Box::new(Failure::new(op, "the engine returned no eid")))
}

impl CommandBody for SpawnStatic {
    type Output = SpawnedStatic;
    const OP: &'static str = "spawn_static";
    const KEY_REQUIRED_IN_DEV: bool = true;
    const SINCE_API_REV: u32 = 1;
    fn target(&self) -> String {
        match &self.spec.name {
            Some(n) => format!("static {n:?} ({})", self.spec.type_name),
            None => format!("static of type {:?}", self.spec.type_name),
        }
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::SpawnStatic(self.spec.clone()))
    }
    fn decode(self, _: &Rc<Session>, r: v1::CommandResult) -> Result<SpawnedStatic> {
        let snap = r.reply.as_ref().and_then(|x| x.entities.first());
        let eid = Eid::from_u64(r.eid)
            .or_else(|| snap.and_then(|e| Eid::from_u64(e.eid)))
            .ok_or_else(|| no_eid(Self::OP))?;
        let ok = match &r.outcome {
            Some(v1::command_result::Outcome::Ok(v)) => Some(v),
            _ => None,
        };
        let name = snap
            .map(|e| e.name.clone())
            .filter(|n| !n.is_empty())
            .or_else(|| {
                ok.and_then(|v| v.get("name"))
                    .and_then(|n| n.as_str())
                    .map(str::to_owned)
            })
            .or(self.spec.name)
            .unwrap_or_default();
        Ok(SpawnedStatic {
            eid,
            name,
            airbase: r
                .reply
                .as_ref()
                .and_then(|x| x.airbases.first())
                .and_then(|a| Eid::from_u64(a.eid)),
            kept: ok
                .and_then(|v| v.get("kept"))
                .and_then(LuaValue::as_bool)
                .unwrap_or(false),
        })
    }
}

/// `ctx.destroy(obj)`: a unit, a static or a cargo.
#[derive(Clone, Debug)]
pub struct DestroyObject {
    object: Obj,
    allow_player: bool,
}

impl DestroyObject {
    pub(crate) fn new(object: Obj) -> Self {
        Self {
            object,
            allow_player: false,
        }
    }
}

impl Cmd<DestroyObject> {
    /// Destroy a unit even when a player occupies it (destroying a player's aircraft leaves
    /// ghosts in multiplayer).
    pub fn allow_player(mut self) -> Self {
        self.body.allow_player = true;
        self
    }
}

impl CommandBody for DestroyObject {
    type Output = ();
    const OP: &'static str = "destroy";
    const KEY_REQUIRED_IN_DEV: bool = true;
    const SINCE_API_REV: u32 = 1;
    fn target(&self) -> String {
        format!("{:?}", self.object.to_proto().sel)
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Destroy(v1::Destroy {
            target: Some(v1::destroy::Target::Object(self.object.to_proto())),
            allow_player: self.allow_player,
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<()> {
        unit(s, r)
    }
}

/// What a [`RemoveJunk`] removed, as DCS counted it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct JunkRemoved {
    pub total: u32,
    /// One count per volume, in order.
    pub per_volume: Vec<u32>,
}

/// `ctx.remove_junk(volumes)`: 1 to 16 DCS volumes. An area volume is accepted only as a
/// circle (or a circular zone) without heights; anything DCS cannot express exactly is
/// `Error::Invalid`.
#[derive(Clone, Debug)]
pub struct RemoveJunk {
    volumes: Vec<Volume>,
}

impl RemoveJunk {
    pub(crate) fn new(volumes: Vec<Volume>) -> Self {
        Self { volumes }
    }
}

impl CommandBody for RemoveJunk {
    type Output = JunkRemoved;
    const OP: &'static str = "remove_junk";
    const SINCE_API_REV: u32 = 1;
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::RemoveJunk(v1::RemoveJunk {
            volumes: self.volumes.iter().map(Volume::to_proto).collect(),
        }))
    }
    fn decode(self, _: &Rc<Session>, r: v1::CommandResult) -> Result<JunkRemoved> {
        let junk = r.reply.and_then(|x| x.junk).unwrap_or_default();
        Ok(JunkRemoved {
            total: junk.total,
            per_volume: junk.per_volume,
        })
    }
}

/// DCS's `Object.Category`: what [`SearchObjects`] looks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ObjectCategory {
    Unit,
    Weapon,
    Static,
    /// Airbases, FARPs and ship decks.
    Base,
    /// Map objects: buildings, bridges and the like of the terrain.
    Scenery,
    Cargo,
}

impl ObjectCategory {
    fn to_proto(self) -> v1::ObjectCategory {
        match self {
            ObjectCategory::Unit => v1::ObjectCategory::Unit,
            ObjectCategory::Weapon => v1::ObjectCategory::Weapon,
            ObjectCategory::Static => v1::ObjectCategory::Static,
            ObjectCategory::Base => v1::ObjectCategory::Base,
            ObjectCategory::Scenery => v1::ObjectCategory::Scenery,
            ObjectCategory::Cargo => v1::ObjectCategory::Cargo,
        }
    }

    fn from_proto(v: i32) -> Option<Self> {
        Some(match v1::ObjectCategory::try_from(v).ok()? {
            v1::ObjectCategory::Unit => ObjectCategory::Unit,
            v1::ObjectCategory::Weapon => ObjectCategory::Weapon,
            v1::ObjectCategory::Static => ObjectCategory::Static,
            v1::ObjectCategory::Base => ObjectCategory::Base,
            v1::ObjectCategory::Scenery => ObjectCategory::Scenery,
            v1::ObjectCategory::Cargo => ObjectCategory::Cargo,
            v1::ObjectCategory::Unspecified => return None,
        })
    }
}

/// One object a [`SearchObjects`] found, as DCS answered when the search ran. A field DCS did
/// not answer is `None` (or empty).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FoundObject {
    pub category: Option<ObjectCategory>,
    /// The world's Eid of a unit, static or cargo it holds: `ctx.world()` has the rest of its
    /// state. `None` for scenery, weapons, airbases and objects the world does not hold.
    pub eid: Option<Eid>,
    /// `getName()`; a map object answers its map id as text.
    pub name: String,
    pub type_name: String,
    pub point: Option<Vec3>,
    /// `getLife()`.
    pub life: Option<f64>,
    /// `getDesc().life`: the life of the undamaged object. `life / life_max` is how intact a
    /// building is (whether DCS keeps the two on one scale for every object is not measured).
    pub life_max: Option<f64>,
    /// `None` for an object without a coalition (scenery).
    pub coalition: Option<Coalition>,
    /// The object's id in DCS (`id_`); for a map object its map id, as the mission editor's
    /// "map object" triggers name it.
    pub dcs_id: u64,
}

/// What a [`SearchObjects`] found.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Found {
    /// In the order DCS handed them over.
    pub objects: Vec<FoundObject>,
    /// The volume held more objects than the limit; the rest is not listed.
    pub truncated: bool,
}

/// `ctx.search_objects(volume)` (API revision 2): `world.searchObjects` over one volume, a
/// read (nothing changes in DCS or in the world). By default it looks for units, statics and
/// scenery and reads at most 500 objects; `.categories(..)` and `.limit(..)` (at most 2000)
/// change that. The volume is a DCS volume type ([`Volume::sphere`], box, segment, pyramid), or
/// a circle area searched as a sphere on the ground; another area is `Error::Invalid`.
#[derive(Clone, Debug)]
pub struct SearchObjects {
    volume: Volume,
    categories: Vec<ObjectCategory>,
    limit: u32,
}

impl SearchObjects {
    pub(crate) fn new(volume: Volume) -> Self {
        Self {
            volume,
            categories: Vec::new(),
            limit: 0,
        }
    }
}

impl Cmd<SearchObjects> {
    /// What to look for (the default: units, statics and scenery).
    pub fn categories(mut self, categories: impl IntoIterator<Item = ObjectCategory>) -> Self {
        self.body.categories = categories.into_iter().collect();
        self
    }

    /// The most objects to read, 1 to 2000 (the default 500). With more in the volume the
    /// result says `truncated`.
    pub fn limit(mut self, n: u32) -> Self {
        self.body.limit = n.max(1);
        self
    }
}

impl CommandBody for SearchObjects {
    type Output = Found;
    const OP: &'static str = "search_objects";
    const SINCE_API_REV: u32 = 2;
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Query(v1::Query {
            kind: Some(v1::query::Kind::SearchObjects(v1::SearchObjects {
                volume: Some(self.volume.to_proto()),
                categories: self
                    .categories
                    .iter()
                    .map(|c| c.to_proto() as i32)
                    .collect(),
                limit: self.limit,
            })),
        }))
    }
    fn decode(self, _: &Rc<Session>, r: v1::CommandResult) -> Result<Found> {
        let found = r.reply.and_then(|x| x.found).unwrap_or_default();
        Ok(Found {
            objects: found
                .objects
                .into_iter()
                .map(|o| FoundObject {
                    category: ObjectCategory::from_proto(o.category),
                    eid: Eid::from_u64(o.eid),
                    name: o.name,
                    type_name: o.type_name,
                    point: o.point.map(Vec3::from),
                    life: o.life,
                    life_max: o.life_max,
                    coalition: Coalition::from_i32(o.coalition),
                    dcs_id: o.dcs_id,
                })
                .collect(),
            truncated: found.truncated,
        })
    }
}
