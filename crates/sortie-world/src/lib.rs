//! The world model, shared by the engine (authoritative **mirror**, allocates [`Eid`]s) and the
//! SDK (**replica** fed by `Start.snapshot`, `Tick.delta` and events).
//!
//! # Invariants
//!
//! * Entities, groups and airbases are keyed by [`Eid`] (`generation << 32 | index`, from a
//!   slotmap). An Eid is never reused for a different object (slots are never freed, so Eid
//!   order is creation order); iteration is ascending Eid (deterministic).
//! * **Revisions.** On the mirror every mutating call that changes something bumps
//!   [`World::rev`] once; each entity/group/airbase remembers the revision of its last change,
//!   so [`World::delta_since`] can produce `WorldDelta`s. Calls that change nothing (a duplicate
//!   BIRTH, a sample of a parked unit) do not bump. A replica's revision only moves with
//!   [`World::apply_snapshot`] / [`World::apply_delta`]: events applied to a replica
//!   ([`World::apply_event`]) change its state but never its revision, so the next tick delta
//!   still applies.
//! * **Death is final.** Once an Eid is dead it never becomes alive again, whatever a later
//!   sample, snapshot or delta says.
//! * **Runtime-id reuse** ([`World::observe`]): a DCS runtime id (`id_`) maps to at most one
//!   entity. An observation of a live object whose runtime id (or name) belongs to an entity
//!   with a different name or type, or to a dead entity, gets a fresh Eid; a live old owner is
//!   retired (marked dead). Same id + same name + same type + alive = same entity (duplicate
//!   BIRTH is harmless). [`World::observe_participant`] is the variant for event participants
//!   that may already be dead (it finds the dead entity instead of creating a ghost).
//! * A group respawned under the same name after all its units died gets a new Eid (and
//!   inherits the manifest ident). Names and idents resolve to the newest object.
//! * Dead entities stay in the world (`dead = true`) until removed ([`World::remove`],
//!   [`World::prune_dead`]); queries skip them unless asked otherwise.
//! * Zones are loaded from the manifest ([`World::set_zones`]); a query with a typed handle uses
//!   the manifest geometry if the ident is known, else the handle's own geometry. Zone queries
//!   use a uniform spatial grid ([`grid::CELL_M`]).

pub mod grid;
mod mirror;
mod model;
mod query;
mod sync;

use std::collections::{BTreeMap, HashMap, HashSet};

use slotmap::{DefaultKey, Key as _, SlotMap};
use sortie_proto::manifest::v1 as mpb;
pub use sortie_types::{AsZone, Category, Coalition, Eid, MapPoint, Vec3, ZoneShape};

pub use mirror::SamplesApplied;
pub use model::{
    Airbase, AsGroupKey, Entity, GroupEntry, GroupKey, Observation, Observed, OwnedShape, Zone,
};
pub use query::{GroupView, QueryIter, UnitQuery};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum WorldError {
    #[error("delta from revision {got} does not apply to revision {have} (resync with a snapshot)")]
    RevisionMismatch { have: u64, got: u64 },
    #[error("snapshot belongs to session {got:?}, world is session {have:?}")]
    WrongSession { have: String, got: String },
    #[error("invalid eid {0}")]
    InvalidEid(u64),
    #[error(
        "changes before revision {floor} were forgotten; a client at revision {from} needs a snapshot"
    )]
    HistoryPruned { from: u64, floor: u64 },
}

/// Which side of the link a world is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Role {
    /// The engine's authoritative mirror: allocates Eids, owns the revision counter.
    #[default]
    Mirror,
    /// The SDK's replica: Eids and revisions come from the engine.
    Replica,
}

#[derive(Clone, Debug, Default)]
struct Sweep {
    id: u64,
    seen: HashSet<Eid>,
}

/// Removal log, for deltas.
#[derive(Clone, Debug, Default)]
struct History {
    /// (revision, eid) of removed entities, ascending revision.
    entities: Vec<(u64, Eid)>,
    groups: Vec<(u64, Eid)>,
    /// Deltas can only be computed from revisions >= floor.
    floor: u64,
}

/// The world: entities, groups, airbases, zones, revisions.
#[derive(Clone, Debug, Default)]
pub struct World {
    session: String,
    role: Role,
    rev: u64,
    model_time: f64,
    alloc: SlotMap<DefaultKey, ()>,
    entities: BTreeMap<Eid, Entity>,
    groups: BTreeMap<Eid, GroupEntry>,
    airbases: BTreeMap<Eid, Airbase>,
    by_runtime: HashMap<u32, Eid>,
    by_name: HashMap<String, Eid>,
    group_by_name: HashMap<String, Eid>,
    group_by_ident: HashMap<String, Eid>,
    airbase_by_name: HashMap<String, Eid>,
    airbase_by_ident: HashMap<String, Eid>,
    zones: BTreeMap<String, Zone>,
    grid: grid::Grid,
    history: History,
    sweep: Option<Sweep>,
}

/// Index helper: `name -> newest eid with that name`.
fn index_insert(index: &mut HashMap<String, Eid>, name: &str, eid: Eid) {
    if name.is_empty() {
        return;
    }
    match index.get_mut(name) {
        Some(current) if *current >= eid => {}
        Some(current) => *current = eid,
        None => {
            index.insert(name.to_owned(), eid);
        }
    }
}

/// Index helper: drops `name -> eid` and falls back to the newest remaining candidate.
fn index_remove(
    index: &mut HashMap<String, Eid>,
    name: &str,
    eid: Eid,
    candidates: impl Iterator<Item = Eid>,
) {
    if index.get(name) == Some(&eid) {
        index.remove(name);
        if let Some(next) = candidates.filter(|e| *e != eid).max() {
            index.insert(name.to_owned(), next);
        }
    }
}

impl World {
    /// An engine mirror for `session`.
    #[must_use]
    pub fn new(session: impl Into<String>) -> Self {
        Self {
            session: session.into(),
            ..Self::default()
        }
    }

    /// An SDK replica for `session` (normally created from `Start.snapshot`, see
    /// [`World::from_snapshot`]).
    #[must_use]
    pub fn replica(session: impl Into<String>) -> Self {
        Self {
            session: session.into(),
            role: Role::Replica,
            ..Self::default()
        }
    }

    /// A replica holding `snap`.
    #[must_use]
    pub fn from_snapshot(snap: &sortie_proto::v1::WorldSnapshot) -> Self {
        let mut w = Self::replica(snap.session.clone());
        w.apply_snapshot(snap);
        w
    }

    #[must_use]
    pub fn role(&self) -> Role {
        self.role
    }

    #[must_use]
    pub fn is_replica(&self) -> bool {
        self.role == Role::Replica
    }

    #[must_use]
    pub fn session(&self) -> &str {
        &self.session
    }

    #[must_use]
    pub fn rev(&self) -> u64 {
        self.rev
    }

    #[must_use]
    pub fn model_time(&self) -> f64 {
        self.model_time
    }

    /// Sets the model time (does not bump the revision).
    pub fn set_model_time(&mut self, t: f64) {
        self.model_time = t;
    }

    /// The revision a change made now gets: the next one on the mirror, the current one on a
    /// replica (whose revision is dictated by the engine).
    pub(crate) fn change_rev(&self) -> u64 {
        match self.role {
            Role::Mirror => self.rev + 1,
            Role::Replica => self.rev,
        }
    }

    /// Commits a change made at `rev` (see [`World::change_rev`]).
    pub(crate) fn commit(&mut self, rev: u64, changed: bool) {
        if changed {
            self.rev = self.rev.max(rev);
        }
    }

    pub(crate) fn alloc_eid(&mut self) -> Eid {
        loop {
            let key = self.alloc.insert(());
            // slotmap keys are `version << 32 | idx` with an odd version >= 1 for occupied
            // slots: exactly the Eid layout. Slots are never freed, so Eids are never reused.
            if let Some(eid) = Eid::from_u64(key.data().as_ffi()) {
                return eid;
            }
        }
    }

    // ----- zones -------------------------------------------------------------------------------

    /// Loads zone geometry from the manifest (replaces all zones).
    pub fn set_zones(&mut self, manifest_zones: &[mpb::Zone]) {
        self.zones = manifest_zones
            .iter()
            .filter_map(Zone::from_manifest)
            .map(|z| (z.ident.clone(), z))
            .collect();
    }

    #[must_use]
    pub fn zone(&self, ident: &str) -> Option<&Zone> {
        self.zones.get(ident)
    }

    pub fn zones(&self) -> impl Iterator<Item = &Zone> {
        self.zones.values()
    }

    /// The live geometry of `zone`: manifest geometry for known idents, else the handle's own.
    pub fn resolve_zone<'a, Z: AsZone + ?Sized>(&'a self, zone: &'a Z) -> ZoneShape<'a> {
        self.zones
            .get(zone.zone_ident())
            .map_or_else(|| zone.zone_shape(), Zone::shape)
    }

    // ----- lookups -----------------------------------------------------------------------------

    #[must_use]
    pub fn entity(&self, eid: Eid) -> Option<&Entity> {
        self.entities.get(&eid)
    }

    /// The newest entity with this DCS name (alive or dead).
    #[must_use]
    pub fn entity_by_name(&self, name: &str) -> Option<&Entity> {
        self.by_name.get(name).and_then(|e| self.entities.get(e))
    }

    /// The entity currently holding this DCS runtime id.
    #[must_use]
    pub fn entity_by_runtime_id(&self, runtime_id: u32) -> Option<&Entity> {
        self.by_runtime
            .get(&runtime_id)
            .and_then(|e| self.entities.get(e))
    }

    /// Every entity (dead ones included), ascending Eid.
    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    /// Every group (respawned predecessors included), ascending Eid.
    pub fn groups(&self) -> impl Iterator<Item = &GroupEntry> {
        self.groups.values()
    }

    #[must_use]
    pub fn group_entry(&self, key: GroupKey<'_>) -> Option<&GroupEntry> {
        let eid = match key {
            GroupKey::Eid(e) => e,
            GroupKey::Name(n) => *self.group_by_name.get(n)?,
            GroupKey::Ident(i) => *self.group_by_ident.get(i)?,
        };
        self.groups.get(&eid)
    }

    pub fn airbases(&self) -> impl Iterator<Item = &Airbase> {
        self.airbases.values()
    }

    #[must_use]
    pub fn airbase(&self, eid: Eid) -> Option<&Airbase> {
        self.airbases.get(&eid)
    }

    #[must_use]
    pub fn airbase_by_name(&self, name: &str) -> Option<&Airbase> {
        self.airbase_by_name
            .get(name)
            .and_then(|e| self.airbases.get(e))
    }

    #[must_use]
    pub fn airbase_by_ident(&self, ident: &str) -> Option<&Airbase> {
        self.airbase_by_ident
            .get(ident)
            .and_then(|e| self.airbases.get(e))
    }

    /// Number of entities (dead ones included).
    #[must_use]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    // ----- queries -----------------------------------------------------------------------------

    /// Live units (airplanes, helicopters, ground units, ships); narrow with the builder.
    #[must_use]
    pub fn units(&self) -> UnitQuery<'_> {
        UnitQuery::new(self)
    }

    /// Live static objects (statics, structures, cargo).
    #[must_use]
    pub fn statics(&self) -> UnitQuery<'_> {
        UnitQuery::new(self).statics()
    }

    /// Live units occupied by players.
    #[must_use]
    pub fn players(&self) -> UnitQuery<'_> {
        UnitQuery::new(self).players()
    }

    /// A view of one group; a missing group behaves as empty (`alive_count() == 0`).
    pub fn group<G: AsGroupKey + ?Sized>(&self, group: &G) -> GroupView<'_> {
        GroupView::new(self, self.group_entry(group.group_key()))
    }

    // ----- index maintenance (shared by mirror and replica code) ---------------------------

    /// Stores `e` (replacing the previous version) and keeps every index in step.
    pub(crate) fn put_entity(&mut self, e: Entity) {
        let eid = e.eid;
        let old = self.entities.get(&eid);
        let old_name = old.map(|o| o.name.clone());
        let old_pos = old.and_then(|o| o.pos);
        if let Some(n) = old_name.filter(|n| *n != e.name) {
            let candidates: Vec<Eid> = self
                .entities
                .values()
                .filter(|x| x.name == n)
                .map(|x| x.eid)
                .collect();
            index_remove(&mut self.by_name, &n, eid, candidates.into_iter());
        }
        index_insert(&mut self.by_name, &e.name, eid);
        if old_pos != e.pos {
            self.grid.set(eid, e.map_point());
        }
        if let Some(g) = e.group
            && let Some(entry) = self.groups.get_mut(&g)
            && !entry.units.contains(&eid)
        {
            entry.units.push(eid);
            entry.rev = entry.rev.max(e.rev);
        }
        self.entities.insert(eid, e);
    }

    pub(crate) fn put_group(&mut self, g: GroupEntry) {
        let eid = g.eid;
        if let Some(old) = self.groups.get(&eid) {
            let (old_name, old_ident) = (old.name.clone(), old.ident.clone());
            if old_name != g.name {
                let c: Vec<Eid> = self.groups_named(&old_name).collect();
                index_remove(&mut self.group_by_name, &old_name, eid, c.into_iter());
            }
            if let Some(i) = old_ident.filter(|i| g.ident.as_ref() != Some(i)) {
                let c: Vec<Eid> = self.groups_with_ident(&i).collect();
                index_remove(&mut self.group_by_ident, &i, eid, c.into_iter());
            }
        }
        index_insert(&mut self.group_by_name, &g.name, eid);
        if let Some(i) = &g.ident {
            index_insert(&mut self.group_by_ident, i, eid);
        }
        self.groups.insert(eid, g);
    }

    pub(crate) fn put_airbase(&mut self, a: Airbase) {
        let eid = a.eid;
        if let Some(old) = self.airbases.get(&eid) {
            let (old_name, old_ident) = (old.name.clone(), old.ident.clone());
            if old_name != a.name {
                let c: Vec<Eid> = self
                    .airbases
                    .values()
                    .filter(|x| x.name == old_name)
                    .map(|x| x.eid)
                    .collect();
                index_remove(&mut self.airbase_by_name, &old_name, eid, c.into_iter());
            }
            if let Some(i) = old_ident.filter(|i| a.ident.as_ref() != Some(i)) {
                let c: Vec<Eid> = self
                    .airbases
                    .values()
                    .filter(|x| x.ident.as_ref() == Some(&i))
                    .map(|x| x.eid)
                    .collect();
                index_remove(&mut self.airbase_by_ident, &i, eid, c.into_iter());
            }
        }
        index_insert(&mut self.airbase_by_name, &a.name, eid);
        if let Some(i) = &a.ident {
            index_insert(&mut self.airbase_by_ident, i, eid);
        }
        self.airbases.insert(eid, a);
    }

    fn groups_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = Eid> + 'a {
        self.groups
            .values()
            .filter(move |g| g.name == name)
            .map(|g| g.eid)
    }

    fn groups_with_ident<'a>(&'a self, ident: &'a str) -> impl Iterator<Item = Eid> + 'a {
        self.groups
            .values()
            .filter(move |g| g.ident.as_deref() == Some(ident))
            .map(|g| g.eid)
    }

    /// Drops an entity and its index entries (no history; callers log removals).
    pub(crate) fn take_entity(&mut self, eid: Eid) -> Option<Entity> {
        let e = self.entities.remove(&eid)?;
        if let Some(rid) = e.runtime_id
            && self.by_runtime.get(&rid) == Some(&eid)
        {
            self.by_runtime.remove(&rid);
        }
        let candidates: Vec<Eid> = self
            .entities
            .values()
            .filter(|x| x.name == e.name)
            .map(|x| x.eid)
            .collect();
        index_remove(&mut self.by_name, &e.name, eid, candidates.into_iter());
        self.grid.remove(eid);
        if let Some(s) = &mut self.sweep {
            s.seen.remove(&eid);
        }
        Some(e)
    }

    pub(crate) fn take_group(&mut self, eid: Eid) -> Option<GroupEntry> {
        let g = self.groups.remove(&eid)?;
        let c: Vec<Eid> = self.groups_named(&g.name).collect();
        index_remove(&mut self.group_by_name, &g.name, eid, c.into_iter());
        if let Some(i) = &g.ident {
            let c: Vec<Eid> = self.groups_with_ident(i).collect();
            index_remove(&mut self.group_by_ident, i, eid, c.into_iter());
        }
        Some(g)
    }

    /// Rebuilds every derived index from the entity/group/airbase maps.
    pub(crate) fn reindex(&mut self) {
        self.by_runtime.clear();
        self.by_name.clear();
        self.group_by_name.clear();
        self.group_by_ident.clear();
        self.airbase_by_name.clear();
        self.airbase_by_ident.clear();
        self.grid.clear();
        for e in self.entities.values() {
            index_insert(&mut self.by_name, &e.name, e.eid);
            if let Some(rid) = e.runtime_id {
                let slot = self.by_runtime.entry(rid).or_insert(e.eid);
                // The newest holder of a runtime id wins.
                if *slot < e.eid {
                    *slot = e.eid;
                }
            }
            self.grid.set(e.eid, e.map_point());
        }
        for g in self.groups.values() {
            index_insert(&mut self.group_by_name, &g.name, g.eid);
            if let Some(i) = &g.ident {
                index_insert(&mut self.group_by_ident, i, g.eid);
            }
        }
        for a in self.airbases.values() {
            index_insert(&mut self.airbase_by_name, &a.name, a.eid);
            if let Some(i) = &a.ident {
                index_insert(&mut self.airbase_by_ident, i, a.eid);
            }
        }
    }
}

#[cfg(test)]
mod tests;
