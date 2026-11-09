//! Read-only queries over a [`World`], used by `ctx.wait_until(|w| ...)` predicates.
//!
//! ```
//! # use sortie_world::*;
//! # use sortie_types::ZoneRef;
//! # #[derive(Clone, Copy, Debug, Default)] struct Op;
//! # const SAM_RING: ZoneRef<Op> = ZoneRef::circle("SAM_RING", MapPoint::new(0.0, 0.0), 60_000.0);
//! # let w = World::new("s");
//! let threat = w.units().coalition(Coalition::Blue).air().any_inside(SAM_RING);
//! let sams = w.units().coalition(Coalition::Red).ground().in_zone(SAM_RING).count();
//! # assert!(!threat && sams == 0);
//! ```
//!
//! Queries are cheap values (a borrow plus a few filters); every filter narrows the set. Dead
//! entities are skipped unless [`UnitQuery::including_dead`] is called. Results come in
//! ascending Eid order. Zone filters use the world's spatial grid.

use std::ops::ControlFlow;
use std::sync::Arc;

use sortie_types::{AsZone, Category, Coalition, Eid, Length, MapPoint, ZoneShape};

use crate::World;
use crate::model::{AsGroupKey, Entity, GroupEntry};

const AIR: &[Category] = &[Category::Airplane, Category::Helicopter];
const PLANES: &[Category] = &[Category::Airplane];
const HELOS: &[Category] = &[Category::Helicopter];
const GROUND: &[Category] = &[Category::GroundUnit];
const SHIPS: &[Category] = &[Category::Ship];
const STATICS: &[Category] = &[Category::Static, Category::Structure, Category::Cargo];
const UNITS: &[Category] = &[
    Category::Airplane,
    Category::Helicopter,
    Category::GroundUnit,
    Category::Ship,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Life {
    Alive,
    Dead,
    Any,
}

/// A zone a query is restricted to.
#[derive(Clone, Debug)]
enum ZoneFilter<'a> {
    Borrowed(ZoneShape<'a>),
    /// A polygon handle not known to the manifest (rare): its vertices are copied.
    Owned(Arc<[sortie_types::MapPoint]>),
}

impl ZoneFilter<'_> {
    fn shape(&self) -> ZoneShape<'_> {
        match self {
            ZoneFilter::Borrowed(s) => *s,
            ZoneFilter::Owned(points) => ZoneShape::Polygon { points },
        }
    }
}

/// Resolves a zone handle against the world: manifest geometry first, then the handle's
/// `'static` geometry, then a copy.
fn zone_filter<'a, Z: AsZone + ?Sized>(world: &'a World, zone: &Z) -> ZoneFilter<'a> {
    if let Some(z) = world.zone(zone.zone_ident()) {
        return ZoneFilter::Borrowed(z.shape());
    }
    if let Some(s) = zone.zone_shape_static() {
        return ZoneFilter::Borrowed(s);
    }
    match zone.zone_shape() {
        ZoneShape::Circle { center, radius_m } => {
            ZoneFilter::Borrowed(ZoneShape::Circle { center, radius_m })
        }
        ZoneShape::Polygon { points } => ZoneFilter::Owned(points.into()),
    }
}

/// A filtered view of the world's entities (see [`World::units`]). By default: live units
/// (airplanes, helicopters, ground units, ships) of every coalition.
#[derive(Clone, Debug)]
pub struct UnitQuery<'a> {
    world: &'a World,
    coalition: Option<Coalition>,
    categories: Option<&'static [Category]>,
    life: Life,
    players_only: bool,
    /// `Some(None)`: restricted to a group the world does not know (matches nothing).
    group: Option<Option<Eid>>,
    type_name: Option<&'a str>,
    zones: Vec<ZoneFilter<'a>>,
}

impl<'a> UnitQuery<'a> {
    /// All live units (airplanes, helicopters, ground units, ships) of the world.
    #[must_use]
    pub fn new(world: &'a World) -> Self {
        Self {
            world,
            coalition: None,
            categories: Some(UNITS),
            life: Life::Alive,
            players_only: false,
            group: None,
            type_name: None,
            zones: Vec::new(),
        }
    }

    #[must_use]
    pub fn coalition(mut self, coalition: Coalition) -> Self {
        self.coalition = Some(coalition);
        self
    }

    /// Airplanes and helicopters.
    #[must_use]
    pub fn air(mut self) -> Self {
        self.categories = Some(AIR);
        self
    }

    #[must_use]
    pub fn planes(mut self) -> Self {
        self.categories = Some(PLANES);
        self
    }

    #[must_use]
    pub fn helos(mut self) -> Self {
        self.categories = Some(HELOS);
        self
    }

    #[must_use]
    pub fn ground(mut self) -> Self {
        self.categories = Some(GROUND);
        self
    }

    #[must_use]
    pub fn ships(mut self) -> Self {
        self.categories = Some(SHIPS);
        self
    }

    /// Static objects, structures and cargo instead of units.
    #[must_use]
    pub fn statics(mut self) -> Self {
        self.categories = Some(STATICS);
        self
    }

    /// Every object category (statics, weapons, ...), not only units.
    #[must_use]
    pub fn any_category(mut self) -> Self {
        self.categories = None;
        self
    }

    /// Only live entities (the default).
    #[must_use]
    pub fn alive(mut self) -> Self {
        self.life = Life::Alive;
        self
    }

    /// Only dead entities (tombstones).
    #[must_use]
    pub fn dead(mut self) -> Self {
        self.life = Life::Dead;
        self
    }

    /// Keeps dead entities (tombstones) in the result.
    #[must_use]
    pub fn including_dead(mut self) -> Self {
        self.life = Life::Any;
        self
    }

    /// Only units occupied by a player.
    #[must_use]
    pub fn players(mut self) -> Self {
        self.players_only = true;
        self
    }

    /// Only members of `group` (Eid, DCS name, `GroupRef`, SDK `Group`). An unknown group
    /// matches nothing.
    #[must_use]
    pub fn in_group<G: AsGroupKey + ?Sized>(mut self, group: &G) -> Self {
        self.group = Some(self.world.group_entry(group.group_key()).map(|g| g.eid));
        self
    }

    /// Only entities of this DCS type, e.g. `"F-16C_50"`.
    #[must_use]
    pub fn of_type(mut self, type_name: &'a str) -> Self {
        self.type_name = Some(type_name);
        self
    }

    /// Only entities with a known position inside `zone` (manifest geometry first). Several
    /// zone filters intersect.
    #[must_use]
    pub fn in_zone<Z: AsZone>(mut self, zone: Z) -> Self {
        let f = zone_filter(self.world, &zone);
        self.zones.push(f);
        self
    }

    /// Only entities within `radius` of `center` on the map.
    #[must_use]
    pub fn within(mut self, center: MapPoint, radius: Length) -> Self {
        self.zones.push(ZoneFilter::Borrowed(ZoneShape::Circle {
            center,
            radius_m: radius.meters(),
        }));
        self
    }

    fn matches_attrs(&self, e: &Entity) -> bool {
        (match self.life {
            Life::Alive => e.is_alive(),
            Life::Dead => e.dead,
            Life::Any => true,
        }) && self.coalition.is_none_or(|c| e.coalition == Some(c))
            && self
                .categories
                .is_none_or(|cats| e.category.is_some_and(|c| cats.contains(&c)))
            && (!self.players_only || e.player.is_some())
            && self.group.is_none_or(|g| g.is_some() && e.group == g)
            && self.type_name.is_none_or(|t| e.type_name == t)
    }

    /// Visits matching entities. With zone filters, candidates come from the spatial grid of
    /// the first zone (in no particular order); otherwise in ascending Eid order.
    fn each(
        &self,
        extra: Option<&ZoneShape<'_>>,
        mut f: impl FnMut(&'a Entity) -> ControlFlow<()>,
    ) {
        let world = self.world;
        let shapes: Vec<ZoneShape<'_>> = self
            .zones
            .iter()
            .map(ZoneFilter::shape)
            .chain(extra.copied())
            .collect();
        let Some((first, rest)) = shapes.split_first() else {
            for e in world.entities.values() {
                if self.matches_attrs(e) && f(e).is_break() {
                    return;
                }
            }
            return;
        };
        world.grid.visit(first, |eid, certainly_inside| {
            let Some(e) = world.entities.get(&eid) else {
                return true;
            };
            let Some(p) = e.map_point() else {
                return true;
            };
            if !self.matches_attrs(e)
                || !(certainly_inside || first.contains(p))
                || !rest.iter().all(|s| s.contains(p))
            {
                return true;
            }
            f(e).is_continue()
        });
    }

    fn collect_sorted(&self, extra: Option<&ZoneShape<'_>>) -> Vec<&'a Entity> {
        let mut out = Vec::new();
        self.each(extra, |e| {
            out.push(e);
            ControlFlow::Continue(())
        });
        if !self.zones.is_empty() || extra.is_some() {
            out.sort_by_key(|e| e.eid);
        }
        out
    }

    /// Matching entities in ascending Eid order.
    #[must_use]
    pub fn iter(&self) -> QueryIter<'a> {
        if self.zones.is_empty() {
            let q = self.clone();
            QueryIter {
                inner: Inner::Scan(Box::new(
                    self.world
                        .entities
                        .values()
                        .filter(move |e| q.matches_attrs(e)),
                )),
            }
        } else {
            QueryIter {
                inner: Inner::List(self.collect_sorted(None).into_iter()),
            }
        }
    }

    /// Eids of the matching entities, ascending.
    #[must_use]
    pub fn eids(&self) -> Vec<Eid> {
        self.iter().map(|e| e.eid).collect()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        let mut n = 0;
        self.each(None, |_| {
            n += 1;
            ControlFlow::Continue(())
        });
        n
    }

    #[must_use]
    pub fn any(&self) -> bool {
        let mut found = false;
        self.each(None, |_| {
            found = true;
            ControlFlow::Break(())
        });
        found
    }

    /// The matching entity with the lowest Eid.
    #[must_use]
    pub fn first(&self) -> Option<&'a Entity> {
        self.iter().next()
    }

    /// The matching entity closest to `p` on the map (ties: lowest Eid).
    #[must_use]
    pub fn nearest_to(&self, p: MapPoint) -> Option<&'a Entity> {
        let mut best: Option<(f64, &'a Entity)> = None;
        self.each(None, |e| {
            if let Some(d) = e.map_point().map(|q| q.distance(p))
                && best.is_none_or(|(bd, be)| d < bd || (d == bd && e.eid < be.eid))
            {
                best = Some((d, e));
            }
            ControlFlow::Continue(())
        });
        best.map(|(_, e)| e)
    }

    /// Matching entities with a known position inside `zone` (manifest geometry first),
    /// ascending Eid.
    pub fn inside<Z: AsZone + ?Sized>(&self, zone: &Z) -> Vec<&'a Entity> {
        let f = zone_filter(self.world, zone);
        self.collect_sorted(Some(&f.shape()))
    }

    /// Is any matching entity inside `zone`?
    #[must_use]
    pub fn any_inside<Z: AsZone>(&self, zone: Z) -> bool {
        let f = zone_filter(self.world, &zone);
        let mut found = false;
        self.each(Some(&f.shape()), |_| {
            found = true;
            ControlFlow::Break(())
        });
        found
    }

    /// Number of matching entities inside `zone`.
    #[must_use]
    pub fn count_inside<Z: AsZone>(&self, zone: Z) -> usize {
        let f = zone_filter(self.world, &zone);
        let mut n = 0;
        self.each(Some(&f.shape()), |_| {
            n += 1;
            ControlFlow::Continue(())
        });
        n
    }
}

impl<'a> IntoIterator for &UnitQuery<'a> {
    type Item = &'a Entity;
    type IntoIter = QueryIter<'a>;

    fn into_iter(self) -> QueryIter<'a> {
        self.iter()
    }
}

impl<'a> IntoIterator for UnitQuery<'a> {
    type Item = &'a Entity;
    type IntoIter = QueryIter<'a>;

    fn into_iter(self) -> QueryIter<'a> {
        self.iter()
    }
}

/// Iterator over query results (ascending Eid).
pub struct QueryIter<'a> {
    inner: Inner<'a>,
}

enum Inner<'a> {
    Scan(Box<dyn Iterator<Item = &'a Entity> + 'a>),
    List(std::vec::IntoIter<&'a Entity>),
}

impl<'a> Iterator for QueryIter<'a> {
    type Item = &'a Entity;

    fn next(&mut self) -> Option<&'a Entity> {
        match &mut self.inner {
            Inner::Scan(it) => it.next(),
            Inner::List(it) => it.next(),
        }
    }
}

impl std::fmt::Debug for QueryIter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("QueryIter")
    }
}

/// A view of one group (see [`World::group`]). A group the world does not know behaves as an
/// empty group: `exists() == false`, `alive_count() == 0`.
#[derive(Clone, Copy, Debug)]
pub struct GroupView<'a> {
    world: &'a World,
    entry: Option<&'a GroupEntry>,
}

impl<'a> GroupView<'a> {
    #[must_use]
    pub fn new(world: &'a World, entry: Option<&'a GroupEntry>) -> Self {
        Self { world, entry }
    }

    #[must_use]
    pub fn exists(&self) -> bool {
        self.entry.is_some()
    }

    #[must_use]
    pub fn eid(&self) -> Option<Eid> {
        self.entry.map(|g| g.eid)
    }

    #[must_use]
    pub fn entry(&self) -> Option<&'a GroupEntry> {
        self.entry
    }

    /// DCS group name (empty for a missing group).
    #[must_use]
    pub fn name(&self) -> &'a str {
        self.entry.map_or("", |g| g.name.as_str())
    }

    /// Manifest symbol, if any.
    #[must_use]
    pub fn ident(&self) -> Option<&'a str> {
        self.entry.and_then(|g| g.ident.as_deref())
    }

    #[must_use]
    pub fn coalition(&self) -> Option<Coalition> {
        self.entry.and_then(|g| g.coalition)
    }

    /// Units of the group still in the world (dead ones included), in group order.
    pub fn units(&self) -> impl Iterator<Item = &'a Entity> + use<'a> {
        let world = self.world;
        self.entry
            .into_iter()
            .flat_map(|g| g.units.iter())
            .filter_map(move |e| world.entities.get(e))
    }

    pub fn alive_units(&self) -> impl Iterator<Item = &'a Entity> + use<'a> {
        self.units().filter(|e| e.is_alive())
    }

    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.alive_units().count()
    }

    /// Number of units the group ever had (pruned ones included).
    #[must_use]
    pub fn size(&self) -> usize {
        self.entry.map_or(0, |g| g.units.len())
    }

    /// True when the group exists, had units, and every one is dead (an empty group is not
    /// "dead"). Units pruned from the world count as dead.
    #[must_use]
    pub fn is_dead(&self) -> bool {
        self.size() > 0 && self.alive_count() == 0
    }

    /// Centre of the live units' positions (map plane), if any has one.
    #[must_use]
    pub fn center(&self) -> Option<MapPoint> {
        let (n, sum) = self
            .alive_units()
            .filter_map(Entity::map_point)
            .fold((0.0, MapPoint::default()), |(n, s), p| {
                (n + 1.0, s.offset(p.north, p.east))
            });
        (n > 0.0).then(|| MapPoint::new(sum.north / n, sum.east / n))
    }

    /// Is any live unit of the group inside `zone`?
    #[must_use]
    pub fn any_inside<Z: AsZone>(&self, zone: Z) -> bool {
        let f = zone_filter(self.world, &zone);
        let shape = f.shape();
        self.alive_units()
            .any(|e| e.map_point().is_some_and(|p| shape.contains(p)))
    }

    /// Number of live units of the group inside `zone`.
    #[must_use]
    pub fn count_inside<Z: AsZone>(&self, zone: Z) -> usize {
        let f = zone_filter(self.world, &zone);
        let shape = f.shape();
        self.alive_units()
            .filter(|e| e.map_point().is_some_and(|p| shape.contains(p)))
            .count()
    }

    /// Are all live units of the group inside `zone` (false for a dead or missing group)?
    #[must_use]
    pub fn all_inside<Z: AsZone>(&self, zone: Z) -> bool {
        let f = zone_filter(self.world, &zone);
        let shape = f.shape();
        let mut any = false;
        self.alive_units().all(|e| {
            any = true;
            e.map_point().is_some_and(|p| shape.contains(p))
        }) && any
    }
}
