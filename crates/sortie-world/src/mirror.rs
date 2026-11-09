//! Engine side: observations from DCS, deaths, sweeps and samples, airbases, pruning.
//!
//! These methods allocate Eids and are meant for the engine's mirror. A replica only uses
//! them in tests.

#[cfg(feature = "link")]
use sortie_proto::link::v1 as link;
use sortie_proto::v1;
use sortie_types::{Category, Coalition, Eid};

use crate::model::{Airbase, Entity, GroupEntry, Observation, Observed};
use crate::{Role, Sweep, World};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The object exists in DCS right now (BIRTH, snapshot, player entering).
    Live,
    /// An event participant that may already be dead (DEAD, KILL target, HIT, ...).
    Participant,
}

#[cfg(feature = "link")]
enum Sampled {
    Updated {
        eid: Eid,
        changed: bool,
        died: bool,
    },
    /// A dead entity reported dead again.
    Wreck,
    Unknown,
}

/// What [`World::apply_samples`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SamplesApplied {
    /// Entities whose position, velocity, heading or liveness changed.
    pub updated: Vec<Eid>,
    /// Entities that the samples reported dead (`alive = false`); also in `updated`.
    pub died: Vec<Eid>,
    /// Runtime ids the world does not know as a live object (the engine should ask DCS about
    /// them, e.g. with a snapshot): DCS spawned something whose BIRTH was missed, or reused
    /// the id of a dead object.
    pub unknown: Vec<u32>,
    /// Live units missing from a closed full sweep, now retired (the engine emits
    /// `UnitGone{MISSING_FROM_SWEEP}` for each).
    pub gone: Vec<Eid>,
}

impl World {
    /// Records a live object seen in DCS (BIRTH initiator, snapshot unit) and returns its Eid;
    /// see the runtime-id reuse rule in the crate docs. Group membership is linked when
    /// `group_name` names a known group.
    pub fn observe(&mut self, obs: &Observation) -> Observed {
        self.observe_in(obs, Mode::Live)
    }

    /// Like [`World::observe`], for event participants that may be dead already (DEAD, KILL,
    /// HIT, UNIT_LOST, ...): an observation matching a dead entity by runtime id or name (same
    /// name and type) resolves to that dead entity instead of creating a new one. Dead
    /// entities stay dead.
    pub fn observe_participant(&mut self, obs: &Observation) -> Observed {
        self.observe_in(obs, Mode::Participant)
    }

    /// The entity an observation refers to, if known (no mutation). Prefers the runtime id,
    /// then the name; the identity (name, type) must match.
    #[must_use]
    pub fn lookup(&self, obs: &Observation) -> Option<Eid> {
        let matches = |e: &Entity| identity_matches(e, obs);
        obs.runtime_id
            .and_then(|r| self.by_runtime.get(&r))
            .and_then(|e| self.entities.get(e))
            .filter(|e| matches(e))
            .or_else(|| {
                obs.name
                    .as_ref()
                    .and_then(|n| self.by_name.get(n))
                    .and_then(|e| self.entities.get(e))
                    .filter(|e| matches(e) && rid_compatible(e, obs))
            })
            .map(|e| e.eid)
    }

    fn observe_in(&mut self, obs: &Observation, mode: Mode) -> Observed {
        let rev = self.change_rev();
        let mut changed = false;
        let mut retired = None;

        let by_rid = obs
            .runtime_id
            .and_then(|r| self.by_runtime.get(&r).copied())
            .and_then(|e| self.entities.get(&e));
        let existing = match by_rid {
            Some(e) if identity_matches(e, obs) && (e.is_alive() || mode == Mode::Participant) => {
                Some(e.eid)
            }
            Some(e) => {
                // The runtime id now belongs to another object (or to a respawn of a dead one).
                let old = e.eid;
                let was_alive = e.is_alive();
                changed |= self.retire(old, rev);
                if was_alive {
                    retired = Some(old);
                }
                None
            }
            None => {
                let by_name = obs
                    .name
                    .as_ref()
                    .and_then(|n| self.by_name.get(n).copied())
                    .and_then(|e| self.entities.get(&e));
                match by_name {
                    Some(x)
                        if identity_matches(x, obs)
                            && rid_compatible(x, obs)
                            && (x.is_alive() || mode == Mode::Participant) =>
                    {
                        Some(x.eid)
                    }
                    Some(x) if x.is_alive() && mode == Mode::Live => {
                        // Same name, different type or runtime id: DCS replaced the object
                        // (coalition.addGroup destroys a group with the same name).
                        let old = x.eid;
                        changed |= self.retire(old, rev);
                        retired = Some(old);
                        None
                    }
                    _ => None,
                }
            }
        };

        let (eid, created) = match existing {
            Some(e) => (e, false),
            None => (self.alloc_eid(), true),
        };
        let group = obs
            .group_name
            .as_ref()
            .and_then(|g| self.group_by_name.get(g).copied());
        let old = self.entities.get(&eid);
        let mut next = old.cloned().unwrap_or_else(|| Entity::new(eid, rev));
        merge_observation(&mut next, obs);
        if group.is_some() {
            next.group = group;
        }
        let group_needs_link = group.is_some_and(|g| {
            self.groups
                .get(&g)
                .is_some_and(|entry| !entry.units.contains(&eid))
        });
        if old.is_none_or(|o| !o.same_state(&next)) || group_needs_link {
            next.rev = rev;
            self.put_entity(next);
            changed = true;
        }
        if let Some(rid) = obs.runtime_id
            && self.by_runtime.get(&rid) != Some(&eid)
        {
            self.by_runtime.insert(rid, eid);
        }
        if let Some(sweep) = &mut self.sweep {
            sweep.seen.insert(eid);
        }
        self.commit(rev, changed);
        match (created, retired) {
            (true, Some(old)) => Observed::Reused { old, new: eid },
            (true, None) => Observed::New(eid),
            (false, _) => Observed::Same(eid),
        }
    }

    /// Records a group seen in DCS (by name). A group whose units are all dead is replaced by a
    /// fresh Eid when observed again (respawn under the same name); the new group inherits the
    /// manifest ident of its predecessor unless `ident` is given.
    pub fn observe_group(
        &mut self,
        name: &str,
        ident: Option<&str>,
        coalition: Option<Coalition>,
        category: Option<Category>,
        group_id: Option<u32>,
    ) -> Eid {
        let rev = self.change_rev();
        let current = self.group_by_name.get(name).copied();
        let reusable = current.filter(|g| {
            self.groups.get(g).is_some_and(|entry| {
                entry.units.is_empty()
                    || entry
                        .units
                        .iter()
                        .any(|u| self.entities.get(u).is_some_and(Entity::is_alive))
            })
        });
        let (eid, mut next) = match reusable.and_then(|g| self.groups.get(&g)) {
            Some(g) => (g.eid, g.clone()),
            None => {
                let eid = self.alloc_eid();
                let mut entry = GroupEntry::new(eid, name, rev);
                // A respawn under the same name keeps the manifest symbol.
                entry.ident = current
                    .and_then(|g| self.groups.get(&g))
                    .and_then(|g| g.ident.clone());
                entry.coalition = current
                    .and_then(|g| self.groups.get(&g))
                    .and_then(|g| g.coalition);
                entry.category = current
                    .and_then(|g| self.groups.get(&g))
                    .and_then(|g| g.category);
                (eid, entry)
            }
        };
        let fresh = reusable.is_none();
        if let Some(i) = ident {
            next.ident = Some(i.to_owned());
        }
        next.coalition = coalition.or(next.coalition);
        next.category = category.or(next.category);
        next.group_id = group_id.or(next.group_id);
        let changed = fresh || self.groups.get(&eid).is_none_or(|g| !g.same_state(&next));
        if changed {
            next.rev = rev;
            self.put_group(next);
        }
        self.commit(rev, changed);
        eid
    }

    /// Marks an entity dead (DEAD, UNIT_LOST, KILL target, UnitGone). Returns `false` if it was
    /// already dead or is unknown. The runtime id stays mapped so later events about the wreck
    /// resolve to it (see [`World::observe_participant`]).
    pub fn mark_dead(&mut self, eid: Eid) -> bool {
        let rev = self.change_rev();
        let t = self.model_time;
        let Some(e) = self.entities.get_mut(&eid) else {
            return false;
        };
        if e.dead {
            return false;
        }
        e.dead = true;
        e.died_at = Some(t);
        e.rev = rev;
        self.commit(rev, true);
        true
    }

    /// Marks dead and forgets the runtime id (the id now belongs to someone else).
    fn retire(&mut self, eid: Eid, rev: u64) -> bool {
        let t = self.model_time;
        let mut changed = false;
        if let Some(e) = self.entities.get_mut(&eid) {
            if !e.dead {
                e.dead = true;
                e.died_at = Some(t);
                e.rev = rev;
                changed = true;
            }
            if let Some(rid) = e.runtime_id
                && self.by_runtime.get(&rid) == Some(&eid)
            {
                self.by_runtime.remove(&rid);
            }
        }
        changed
    }

    /// Removes an entity entirely (it appears in `removals` of the next delta).
    pub fn remove(&mut self, eid: Eid) -> Option<Entity> {
        let rev = self.change_rev();
        let e = self.take_entity(eid)?;
        if self.role == Role::Mirror {
            self.history.entities.push((rev, eid));
        }
        self.commit(rev, true);
        Some(e)
    }

    /// Removes a group entry (its units stay; see [`World::prune_dead`]).
    pub fn remove_group(&mut self, eid: Eid) -> Option<GroupEntry> {
        let rev = self.change_rev();
        let g = self.take_group(eid)?;
        if self.role == Role::Mirror {
            self.history.groups.push((rev, eid));
        }
        self.commit(rev, true);
        Some(g)
    }

    /// Tombstone pruning: removes entities that died at least `keep_dead_s` seconds of model
    /// time ago, and runtime groups (no manifest ident) none of whose units remain. Manifest
    /// groups are kept forever (an empty group still answers `alive_count() == 0`). Returns
    /// the removed entity Eids.
    pub fn prune_dead(&mut self, keep_dead_s: f64) -> Vec<Eid> {
        let now = self.model_time;
        let rev = self.change_rev();
        let doomed: Vec<Eid> = self
            .entities
            .values()
            .filter(|e| e.dead && e.died_at.is_some_and(|t| now - t >= keep_dead_s))
            .map(|e| e.eid)
            .collect();
        let mirror = self.role == Role::Mirror;
        for e in &doomed {
            self.take_entity(*e);
            if mirror {
                self.history.entities.push((rev, *e));
            }
        }
        let empty_groups: Vec<Eid> = self
            .groups
            .values()
            .filter(|g| {
                g.ident.is_none()
                    && !g.units.is_empty()
                    && g.units.iter().all(|u| !self.entities.contains_key(u))
            })
            .map(|g| g.eid)
            .collect();
        for g in &empty_groups {
            self.take_group(*g);
            if mirror {
                self.history.groups.push((rev, *g));
            }
        }
        self.commit(rev, !doomed.is_empty() || !empty_groups.is_empty());
        doomed
    }

    /// Forgets the removal log up to and including `rev` (every client has seen it). Deltas
    /// from earlier revisions then fail with [`crate::WorldError::HistoryPruned`].
    pub fn forget_history_through(&mut self, rev: u64) {
        let rev = rev.min(self.rev);
        self.history.entities.retain(|(r, _)| *r > rev);
        self.history.groups.retain(|(r, _)| *r > rev);
        self.history.floor = self.history.floor.max(rev);
    }

    // ----- sweeps and samples ----------------------------------------------------------------

    /// Starts tracking which entities a full sample sweep (or snapshot) sees.
    pub fn begin_sweep(&mut self, sweep: u64) {
        self.sweep = Some(Sweep {
            id: sweep,
            seen: Default::default(),
        });
    }

    /// Ends a sweep: live entities with a runtime id that matched `in_scope` but were not seen
    /// are retired and returned (the engine emits `UnitGone` for each). A sweep id that does
    /// not match the open sweep is ignored.
    pub fn end_sweep(&mut self, sweep: u64, in_scope: impl Fn(&Entity) -> bool) -> Vec<Eid> {
        if self.sweep.as_ref().is_none_or(|s| s.id != sweep) {
            return Vec::new();
        }
        let Some(Sweep { seen, .. }) = self.sweep.take() else {
            return Vec::new();
        };
        let gone: Vec<Eid> = self
            .entities
            .values()
            .filter(|e| {
                e.is_alive() && e.runtime_id.is_some() && in_scope(e) && !seen.contains(&e.eid)
            })
            .map(|e| e.eid)
            .collect();
        let rev = self.change_rev();
        let mut changed = false;
        for e in &gone {
            changed |= self.retire(*e, rev);
        }
        self.commit(rev, changed);
        gone
    }

    /// Applies one link `Sample` (position update by runtime id). Returns the Eid of the live
    /// entity it updated; unknown runtime ids and ids of dead entities give `None` (for a
    /// `alive = true` sample the engine should ask DCS about the object).
    #[cfg(feature = "link")]
    pub fn apply_sample(&mut self, s: &link::Sample) -> Option<Eid> {
        let rev = self.change_rev();
        match self.sample_at(s, rev) {
            Sampled::Updated { eid, changed, .. } => {
                self.commit(rev, changed);
                Some(eid)
            }
            Sampled::Wreck | Sampled::Unknown => None,
        }
    }

    #[cfg(feature = "link")]
    fn sample_at(&mut self, s: &link::Sample, rev: u64) -> Sampled {
        let Some(eid) = self.by_runtime.get(&s.runtime_id).copied() else {
            return Sampled::Unknown;
        };
        let t = self.model_time;
        let Some(old) = self.entities.get(&eid) else {
            return Sampled::Unknown;
        };
        if old.dead {
            // A wreck still answering, or a new object that took over the id unseen.
            return if s.alive {
                Sampled::Unknown
            } else {
                Sampled::Wreck
            };
        }
        let mut next = old.clone();
        next.pos = s.pos.map(Into::into).or(next.pos);
        next.velocity = s.velocity.map(Into::into).or(next.velocity);
        next.heading = Some(s.heading_rad);
        let died = !s.alive;
        if died {
            next.dead = true;
            next.died_at = Some(t);
        }
        if let Some(sweep) = &mut self.sweep {
            sweep.seen.insert(eid);
        }
        if old.same_state(&next) {
            return Sampled::Updated {
                eid,
                changed: false,
                died: false,
            };
        }
        next.rev = rev;
        self.put_entity(next);
        Sampled::Updated {
            eid,
            changed: true,
            died,
        }
    }

    /// Applies one `Samples` frame as a single revision. A frame of a full sweep opens the
    /// sweep if needed; the frame with `sweep_end` of a full sweep closes it and retires the
    /// live units (with a runtime id) that it did not see. Statics and weapons are not sampled
    /// and never retired by a sweep.
    #[cfg(feature = "link")]
    pub fn apply_samples(&mut self, frame: &link::Samples) -> SamplesApplied {
        let mut out = SamplesApplied::default();
        self.model_time = self.model_time.max(frame.model_time);
        if frame.full_sweep && self.sweep.as_ref().is_none_or(|s| s.id != frame.sweep) {
            self.begin_sweep(frame.sweep);
        }
        let rev = self.change_rev();
        let mut changed = false;
        for s in &frame.samples {
            match self.sample_at(s, rev) {
                Sampled::Updated {
                    eid,
                    changed: c,
                    died,
                } => {
                    if c {
                        changed = true;
                        out.updated.push(eid);
                    }
                    if died {
                        out.died.push(eid);
                    }
                }
                Sampled::Wreck => {}
                Sampled::Unknown => out.unknown.push(s.runtime_id),
            }
        }
        self.commit(rev, changed);
        if frame.full_sweep && frame.sweep_end {
            out.gone = self.end_sweep(frame.sweep, |e| e.category.is_none_or(Category::is_unit));
        }
        out
    }

    // ----- airbases --------------------------------------------------------------------------

    /// Inserts or updates an airbase, matched by DCS name. The snap's `eid` is ignored on the
    /// mirror (the world allocates one) and used as given on a replica.
    pub fn upsert_airbase(&mut self, a: v1::AirbaseSnap) -> Eid {
        let rev = self.change_rev();
        let eid = match self.role {
            Role::Replica => Eid::from_u64(a.eid),
            Role::Mirror => None,
        }
        .or_else(|| self.airbase_by_name.get(&a.name).copied())
        .unwrap_or_else(|| self.alloc_eid());
        let mut next = Airbase {
            eid,
            name: a.name,
            ident: (!a.ident.is_empty()).then_some(a.ident),
            coalition: Coalition::from_i32(a.coalition),
            pos: a.pos.map(Into::into),
            airbase_id: (a.airbase_id != 0).then_some(a.airbase_id),
            rev,
        };
        let old = self.airbases.get(&eid);
        if let Some(o) = old {
            // Keep what the update does not know.
            next.ident = next.ident.or_else(|| o.ident.clone());
            next.pos = next.pos.or(o.pos);
            next.airbase_id = next.airbase_id.or(o.airbase_id);
            next.coalition = next.coalition.or(o.coalition);
        }
        let changed = old.is_none_or(|o| !o.same_state(&next));
        if changed {
            self.put_airbase(next);
        }
        self.commit(rev, changed);
        eid
    }

    /// Changes an airbase's owner (BaseCaptured). Returns `false` if unknown or unchanged.
    pub fn set_airbase_coalition(&mut self, eid: Eid, coalition: Option<Coalition>) -> bool {
        let rev = self.change_rev();
        let Some(a) = self.airbases.get_mut(&eid) else {
            return false;
        };
        if a.coalition == coalition {
            return false;
        }
        a.coalition = coalition;
        a.rev = rev;
        self.commit(rev, true);
        true
    }
}

fn identity_matches(e: &Entity, obs: &Observation) -> bool {
    obs.name
        .as_deref()
        .is_none_or(|n| e.name.is_empty() || n == e.name)
        && obs
            .type_name
            .as_deref()
            .is_none_or(|t| e.type_name.is_empty() || t == e.type_name)
}

/// A name match is only the same object if the runtime ids do not contradict each other.
fn rid_compatible(e: &Entity, obs: &Observation) -> bool {
    match (e.runtime_id, obs.runtime_id) {
        (Some(a), Some(b)) => a == b,
        _ => true,
    }
}

fn merge_observation(e: &mut Entity, obs: &Observation) {
    macro_rules! set_string {
        ($field:ident) => {
            if let Some(v) = &obs.$field {
                e.$field.clone_from(v);
            }
        };
    }
    set_string!(name);
    set_string!(type_name);
    set_string!(group_name);
    set_string!(country);
    macro_rules! set_opt {
        ($($field:ident),*) => {$(
            if obs.$field.is_some() {
                e.$field = obs.$field;
            }
        )*};
    }
    set_opt!(
        coalition, category, pos, velocity, heading, life, unit_id, runtime_id
    );
    if obs.player.is_some() {
        e.player.clone_from(&obs.player);
    }
}
