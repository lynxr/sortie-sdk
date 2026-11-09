//! Plain data of the world: entities, groups, airbases, zones, observations, group keys.

use std::sync::Arc;

use sortie_proto::manifest::v1 as mpb;
use sortie_proto::v1;
use sortie_types::{AsZone, Category, Coalition, Eid, MapPoint, Vec3, ZoneShape};

/// One object: unit, static, weapon...
#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub eid: Eid,
    pub name: String,
    pub type_name: String,
    pub coalition: Option<Coalition>,
    pub category: Option<Category>,
    pub group: Option<Eid>,
    pub group_name: String,
    pub country: String,
    pub pos: Option<Vec3>,
    pub velocity: Option<Vec3>,
    pub heading: Option<f64>,
    pub player: Option<String>,
    /// Last known DCS object handle (`id_`). Informational: DCS reuses these.
    pub runtime_id: Option<u32>,
    pub unit_id: Option<u32>,
    pub life: Option<f64>,
    /// Death is final for an Eid: nothing ever sets it back to `false`.
    pub dead: bool,
    /// Model time at which the mirror learned of the death (used for tombstone pruning).
    pub died_at: Option<f64>,
    /// Revision of the last change.
    pub rev: u64,
}

impl Entity {
    /// A blank, live entity.
    #[must_use]
    pub fn new(eid: Eid, rev: u64) -> Self {
        Self {
            eid,
            name: String::new(),
            type_name: String::new(),
            coalition: None,
            category: None,
            group: None,
            group_name: String::new(),
            country: String::new(),
            pos: None,
            velocity: None,
            heading: None,
            player: None,
            runtime_id: None,
            unit_id: None,
            life: None,
            dead: false,
            died_at: None,
            rev,
        }
    }

    #[must_use]
    pub fn is_alive(&self) -> bool {
        !self.dead
    }

    #[must_use]
    pub fn map_point(&self) -> Option<MapPoint> {
        self.pos.map(Vec3::map_point)
    }

    /// Is this a unit (airplane, helicopter, ground unit or ship)?
    #[must_use]
    pub fn is_unit(&self) -> bool {
        self.category.is_some_and(Category::is_unit)
    }

    #[must_use]
    pub fn to_snap(&self) -> v1::EntitySnap {
        v1::EntitySnap {
            eid: self.eid.to_u64(),
            name: self.name.clone(),
            type_name: self.type_name.clone(),
            coalition: self.coalition.map_or(0, Coalition::as_i32),
            category: self.category.map_or(0, Category::as_i32),
            pos: self.pos.map(Into::into),
            player: self.player.clone(),
            group_eid: self.group.map(Eid::to_u64),
            country: self.country.clone(),
            velocity: self.velocity.map(Into::into),
            heading_rad: self.heading,
            dead: self.dead,
            runtime_id: self.runtime_id.unwrap_or(0),
            unit_id: self.unit_id.unwrap_or(0),
            group_name: self.group_name.clone(),
            life: self.life.unwrap_or(0.0),
        }
    }

    /// `None` if the snap has no valid eid.
    #[must_use]
    pub fn from_snap(s: &v1::EntitySnap, rev: u64) -> Option<Self> {
        Some(Self {
            eid: Eid::from_u64(s.eid)?,
            name: s.name.clone(),
            type_name: s.type_name.clone(),
            coalition: Coalition::from_i32(s.coalition),
            category: Category::from_i32(s.category),
            group: s.group_eid.and_then(Eid::from_u64),
            group_name: s.group_name.clone(),
            country: s.country.clone(),
            pos: s.pos.map(Into::into),
            velocity: s.velocity.map(Into::into),
            heading: s.heading_rad,
            player: s.player.clone(),
            runtime_id: (s.runtime_id != 0).then_some(s.runtime_id),
            unit_id: (s.unit_id != 0).then_some(s.unit_id),
            life: (s.life != 0.0).then_some(s.life),
            dead: s.dead,
            died_at: None,
            rev,
        })
    }

    /// Equal in everything a snapshot carries (ignores `rev` and `died_at`).
    pub(crate) fn same_state(&self, other: &Entity) -> bool {
        Entity {
            rev: other.rev,
            died_at: other.died_at,
            ..self.clone()
        } == *other
    }
}

/// A group.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupEntry {
    pub eid: Eid,
    pub name: String,
    /// Manifest symbol, if the group comes from the manifest.
    pub ident: Option<String>,
    pub coalition: Option<Coalition>,
    pub category: Option<Category>,
    /// Unit Eids in DCS order. A unit that is no longer in the world (pruned) counts as dead.
    pub units: Vec<Eid>,
    pub group_id: Option<u32>,
    pub country: String,
    pub rev: u64,
}

impl GroupEntry {
    #[must_use]
    pub fn new(eid: Eid, name: &str, rev: u64) -> Self {
        Self {
            eid,
            name: name.to_owned(),
            ident: None,
            coalition: None,
            category: None,
            units: Vec::new(),
            group_id: None,
            country: String::new(),
            rev,
        }
    }

    #[must_use]
    pub fn to_snap(&self) -> v1::GroupSnap {
        v1::GroupSnap {
            eid: self.eid.to_u64(),
            name: self.name.clone(),
            ident: self.ident.clone().unwrap_or_default(),
            coalition: self.coalition.map_or(0, Coalition::as_i32),
            category: self.category.map_or(0, Category::as_i32),
            units: self.units.iter().map(|e| e.to_u64()).collect(),
            group_id: self.group_id.unwrap_or(0),
            country: self.country.clone(),
        }
    }

    #[must_use]
    pub fn from_snap(s: &v1::GroupSnap, rev: u64) -> Option<Self> {
        Some(Self {
            eid: Eid::from_u64(s.eid)?,
            name: s.name.clone(),
            ident: (!s.ident.is_empty()).then(|| s.ident.clone()),
            coalition: Coalition::from_i32(s.coalition),
            category: Category::from_i32(s.category),
            units: s.units.iter().filter_map(|e| Eid::from_u64(*e)).collect(),
            group_id: (s.group_id != 0).then_some(s.group_id),
            country: s.country.clone(),
            rev,
        })
    }

    pub(crate) fn same_state(&self, other: &GroupEntry) -> bool {
        GroupEntry {
            rev: other.rev,
            ..self.clone()
        } == *other
    }
}

/// An airbase (airfield, FARP, carrier) as tracked by the world.
#[derive(Clone, Debug, PartialEq)]
pub struct Airbase {
    pub eid: Eid,
    /// DCS name, e.g. "Maykop-Khanskaya".
    pub name: String,
    /// Manifest symbol, e.g. "MAYKOP_KHANSKAYA".
    pub ident: Option<String>,
    /// Current owner (updated by BaseCaptured).
    pub coalition: Option<Coalition>,
    pub pos: Option<Vec3>,
    /// `Airbase:getID()` (airdromeId for airfields).
    pub airbase_id: Option<u32>,
    pub rev: u64,
}

impl Airbase {
    #[must_use]
    pub fn to_snap(&self) -> v1::AirbaseSnap {
        v1::AirbaseSnap {
            eid: self.eid.to_u64(),
            name: self.name.clone(),
            ident: self.ident.clone().unwrap_or_default(),
            coalition: self.coalition.map_or(0, Coalition::as_i32),
            pos: self.pos.map(Into::into),
            airbase_id: self.airbase_id.unwrap_or(0),
        }
    }

    #[must_use]
    pub fn from_snap(s: &v1::AirbaseSnap, rev: u64) -> Option<Self> {
        Some(Self {
            eid: Eid::from_u64(s.eid)?,
            name: s.name.clone(),
            ident: (!s.ident.is_empty()).then(|| s.ident.clone()),
            coalition: Coalition::from_i32(s.coalition),
            pos: s.pos.map(Into::into),
            airbase_id: (s.airbase_id != 0).then_some(s.airbase_id),
            rev,
        })
    }

    pub(crate) fn same_state(&self, other: &Airbase) -> bool {
        Airbase {
            rev: other.rev,
            ..self.clone()
        } == *other
    }
}

/// A zone with owned geometry (from the manifest).
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub ident: String,
    pub name: String,
    pub shape: OwnedShape,
}

/// Owned zone geometry. Polygons share their vertices (cheap to clone into queries).
#[derive(Clone, Debug, PartialEq)]
pub enum OwnedShape {
    Circle { center: MapPoint, radius_m: f64 },
    Polygon(Arc<[MapPoint]>),
}

impl OwnedShape {
    #[must_use]
    pub fn shape(&self) -> ZoneShape<'_> {
        match self {
            OwnedShape::Circle { center, radius_m } => ZoneShape::Circle {
                center: *center,
                radius_m: *radius_m,
            },
            OwnedShape::Polygon(points) => ZoneShape::Polygon { points },
        }
    }

    #[must_use]
    pub fn from_shape(shape: ZoneShape<'_>) -> Self {
        match shape {
            ZoneShape::Circle { center, radius_m } => OwnedShape::Circle { center, radius_m },
            ZoneShape::Polygon { points } => OwnedShape::Polygon(points.into()),
        }
    }
}

impl Zone {
    #[must_use]
    pub fn shape(&self) -> ZoneShape<'_> {
        self.shape.shape()
    }

    /// `None` when the manifest zone has no shape.
    #[must_use]
    pub fn from_manifest(z: &mpb::Zone) -> Option<Self> {
        let shape = match z.shape.as_ref()? {
            mpb::zone::Shape::Circle(c) => OwnedShape::Circle {
                center: c.center.map(Into::into).unwrap_or_default(),
                radius_m: c.radius_m,
            },
            mpb::zone::Shape::Polygon(p) => {
                OwnedShape::Polygon(p.points.iter().copied().map(Into::into).collect())
            }
        };
        Some(Self {
            ident: z.ident.clone(),
            name: z.name.clone(),
            shape,
        })
    }
}

impl AsZone for Zone {
    fn zone_ident(&self) -> &str {
        &self.ident
    }

    fn zone_shape(&self) -> ZoneShape<'_> {
        self.shape()
    }
}

/// What the engine learned about an object from DCS (raw event participant, snapshot unit).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Observation {
    pub runtime_id: Option<u32>,
    pub unit_id: Option<u32>,
    pub name: Option<String>,
    pub type_name: Option<String>,
    pub coalition: Option<Coalition>,
    pub category: Option<Category>,
    pub group_name: Option<String>,
    pub country: Option<String>,
    pub player: Option<String>,
    pub pos: Option<Vec3>,
    pub velocity: Option<Vec3>,
    pub heading: Option<f64>,
    pub life: Option<f64>,
}

/// Result of [`crate::World::observe`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observed {
    /// A new entity.
    New(Eid),
    /// An entity already known (fields updated).
    Same(Eid),
    /// The runtime id or name belonged to the live entity `old`, which was retired (marked
    /// dead); `new` was allocated.
    Reused { old: Eid, new: Eid },
}

impl Observed {
    #[must_use]
    pub fn eid(self) -> Eid {
        match self {
            Observed::New(e) | Observed::Same(e) | Observed::Reused { new: e, .. } => e,
        }
    }

    /// The entity retired by this observation, if any (the engine emits
    /// `UnitGone{RUNTIME_ID_REUSED}` for it).
    #[must_use]
    pub fn retired(self) -> Option<Eid> {
        match self {
            Observed::Reused { old, .. } => Some(old),
            _ => None,
        }
    }
}

/// How to find a group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupKey<'a> {
    Eid(Eid),
    /// DCS group name (resolves to the newest group with that name).
    Name(&'a str),
    /// Manifest symbol (resolves to the newest group carrying it).
    Ident(&'a str),
}

/// Anything that identifies a group: `Eid`, a DCS name, a `GroupRef` handle, SDK `Group<K>`.
pub trait AsGroupKey {
    fn group_key(&self) -> GroupKey<'_>;
}

impl AsGroupKey for GroupKey<'_> {
    fn group_key(&self) -> GroupKey<'_> {
        *self
    }
}

impl AsGroupKey for Eid {
    fn group_key(&self) -> GroupKey<'_> {
        GroupKey::Eid(*self)
    }
}

impl AsGroupKey for str {
    fn group_key(&self) -> GroupKey<'_> {
        GroupKey::Name(self)
    }
}

impl AsGroupKey for String {
    fn group_key(&self) -> GroupKey<'_> {
        GroupKey::Name(self)
    }
}

impl<M, K> AsGroupKey for sortie_types::GroupRef<M, K> {
    fn group_key(&self) -> GroupKey<'_> {
        GroupKey::Ident(self.ident())
    }
}

impl<T: AsGroupKey + ?Sized> AsGroupKey for &T {
    fn group_key(&self) -> GroupKey<'_> {
        (**self).group_key()
    }
}
