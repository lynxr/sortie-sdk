//! Both sides: snapshots, deltas, diffs and events.

use sortie_proto::v1;
use sortie_types::{Coalition, Eid};

use crate::model::{Airbase, Entity, GroupEntry};
use crate::{Role, World, WorldError};

impl World {
    /// Full snapshot of the current state (ascending Eid everywhere).
    #[must_use]
    pub fn snapshot(&self) -> v1::WorldSnapshot {
        v1::WorldSnapshot {
            session: self.session.clone(),
            rev: self.rev,
            model_time: self.model_time,
            groups: self.groups.values().map(GroupEntry::to_snap).collect(),
            entities: self.entities.values().map(Entity::to_snap).collect(),
            airbases: self.airbases.values().map(Airbase::to_snap).collect(),
        }
    }

    /// Replica side: replaces the whole state with `snap` (Eids as given). Zones are kept.
    /// The world becomes a replica.
    pub fn apply_snapshot(&mut self, snap: &v1::WorldSnapshot) {
        let zones = std::mem::take(&mut self.zones);
        *self = World {
            session: snap.session.clone(),
            role: Role::Replica,
            rev: snap.rev,
            model_time: snap.model_time,
            zones,
            ..World::default()
        };
        let t = snap.model_time;
        self.groups = snap
            .groups
            .iter()
            .filter_map(|g| GroupEntry::from_snap(g, snap.rev))
            .map(|g| (g.eid, g))
            .collect();
        self.entities = snap
            .entities
            .iter()
            .filter_map(|e| Entity::from_snap(e, snap.rev))
            .map(|mut e| {
                if e.dead {
                    e.died_at = Some(t);
                }
                (e.eid, e)
            })
            .collect();
        self.airbases = snap
            .airbases
            .iter()
            .filter_map(|a| Airbase::from_snap(a, snap.rev))
            .map(|a| (a.eid, a))
            .collect();
        self.reindex();
    }

    /// Changes since `from_rev` (entities/groups/airbases changed after it, removals after it).
    /// If the removal log before `from_rev` was forgotten the delta may miss removals; the
    /// engine should use [`World::try_delta_since`].
    #[must_use]
    pub fn delta_since(&self, from_rev: u64) -> v1::WorldDelta {
        let removed = |log: &[(u64, Eid)]| -> Vec<u64> {
            let start = log.partition_point(|(r, _)| *r <= from_rev);
            log[start..].iter().map(|(_, e)| e.to_u64()).collect()
        };
        v1::WorldDelta {
            from_rev,
            rev: self.rev,
            model_time: self.model_time,
            upserts: self
                .entities
                .values()
                .filter(|e| e.rev > from_rev)
                .map(Entity::to_snap)
                .collect(),
            removals: removed(&self.history.entities),
            group_upserts: self
                .groups
                .values()
                .filter(|g| g.rev > from_rev)
                .map(GroupEntry::to_snap)
                .collect(),
            group_removals: removed(&self.history.groups),
            airbase_upserts: self
                .airbases
                .values()
                .filter(|a| a.rev > from_rev)
                .map(Airbase::to_snap)
                .collect(),
        }
    }

    /// [`World::delta_since`], refusing revisions older than the forgotten history (the client
    /// then needs a fresh snapshot) and revisions from the future.
    pub fn try_delta_since(&self, from_rev: u64) -> Result<v1::WorldDelta, WorldError> {
        if from_rev < self.history.floor {
            return Err(WorldError::HistoryPruned {
                from: from_rev,
                floor: self.history.floor,
            });
        }
        if from_rev > self.rev {
            return Err(WorldError::RevisionMismatch {
                have: self.rev,
                got: from_rev,
            });
        }
        Ok(self.delta_since(from_rev))
    }

    /// Has anything changed after `rev`?
    #[must_use]
    pub fn changed_since(&self, rev: u64) -> bool {
        self.rev > rev
    }

    /// Runs `f` on the mirror and returns its result with the delta of what it changed:
    /// `let (eid, delta) = world.track(|w| w.observe(&obs).eid());`.
    pub fn track<R>(&mut self, f: impl FnOnce(&mut World) -> R) -> (R, v1::WorldDelta) {
        let before = self.rev;
        let r = f(self);
        (r, self.delta_since(before))
    }

    /// The delta that turns `self` into `target` (two states of the same world, e.g. the
    /// mirror before and after a batch): applying it to a replica holding `self` yields
    /// `target`'s state. Computed by comparison, so it needs no history.
    #[must_use]
    pub fn diff(&self, target: &World) -> v1::WorldDelta {
        fn changed<'a, T: 'a, S>(
            from: &std::collections::BTreeMap<Eid, T>,
            to: &'a std::collections::BTreeMap<Eid, T>,
            same: impl Fn(&T, &T) -> bool,
            snap: impl Fn(&T) -> S,
        ) -> Vec<S> {
            to.iter()
                .filter(|(eid, x)| from.get(eid).is_none_or(|old| !same(old, x)))
                .map(|(_, x)| snap(x))
                .collect()
        }
        fn gone<T, U>(
            from: &std::collections::BTreeMap<Eid, T>,
            to: &std::collections::BTreeMap<Eid, U>,
        ) -> Vec<u64> {
            from.keys()
                .filter(|e| !to.contains_key(e))
                .map(|e| e.to_u64())
                .collect()
        }
        v1::WorldDelta {
            from_rev: self.rev,
            rev: target.rev,
            model_time: target.model_time,
            upserts: changed(
                &self.entities,
                &target.entities,
                Entity::same_state,
                Entity::to_snap,
            ),
            removals: gone(&self.entities, &target.entities),
            group_upserts: changed(
                &self.groups,
                &target.groups,
                GroupEntry::same_state,
                GroupEntry::to_snap,
            ),
            group_removals: gone(&self.groups, &target.groups),
            airbase_upserts: changed(
                &self.airbases,
                &target.airbases,
                Airbase::same_state,
                Airbase::to_snap,
            ),
        }
    }

    /// Replica side: applies a delta that starts at the current revision. An empty delta
    /// (nothing changed) applies at any revision. The world becomes a replica.
    pub fn apply_delta(&mut self, d: &v1::WorldDelta) -> Result<(), WorldError> {
        let empty = d.upserts.is_empty()
            && d.removals.is_empty()
            && d.group_upserts.is_empty()
            && d.group_removals.is_empty()
            && d.airbase_upserts.is_empty();
        if d.from_rev != self.rev && !empty {
            return Err(WorldError::RevisionMismatch {
                have: self.rev,
                got: d.from_rev,
            });
        }
        self.role = Role::Replica;
        let rev = d.rev.max(self.rev);
        self.rev = rev;
        self.model_time = d.model_time;
        for g in &d.group_upserts {
            if let Some(g) = GroupEntry::from_snap(g, rev) {
                self.put_group(g);
            }
        }
        for s in &d.upserts {
            self.upsert_snap(s, rev);
        }
        for r in &d.removals {
            if let Some(eid) = Eid::from_u64(*r) {
                self.take_entity(eid);
            }
        }
        for r in &d.group_removals {
            if let Some(eid) = Eid::from_u64(*r) {
                self.take_group(eid);
            }
        }
        for a in &d.airbase_upserts {
            if let Some(a) = Airbase::from_snap(a, rev) {
                self.put_airbase(a);
            }
        }
        Ok(())
    }

    /// Inserts or replaces an entity from a snap (Eid as given), keeping death final and the
    /// runtime-id index pointing at the newest holder. Returns the Eid and whether the state
    /// changed.
    fn upsert_snap(&mut self, s: &v1::EntitySnap, rev: u64) -> Option<(Eid, bool)> {
        let mut e = Entity::from_snap(s, rev)?;
        let eid = e.eid;
        let old = self.entities.get(&eid);
        if let Some(o) = old {
            if o.dead {
                e.dead = true;
            }
            e.died_at = o.died_at;
        }
        if e.dead && e.died_at.is_none() {
            e.died_at = Some(self.model_time);
        }
        let changed = old.is_none_or(|o| !o.same_state(&e));
        if let Some(rid) = e.runtime_id {
            let slot = self.by_runtime.entry(rid).or_insert(eid);
            if *slot < eid {
                *slot = eid;
            }
        }
        if changed {
            self.put_entity(e);
        }
        Some((eid, changed))
    }

    /// Applies an event to the world: participants are upserted from their snaps, Dead /
    /// UnitLost / Kill (target) / UnitGone mark the entity dead, BaseCaptured changes the
    /// airbase owner, PlayerEnteredUnit records the player. On a replica this never changes
    /// the revision (the engine's next delta carries the same facts); on the mirror it bumps
    /// it when something changed. The SDK calls this before user code sees the event.
    pub fn apply_event(&mut self, ev: &v1::Event) {
        use v1::event::Kind;
        self.model_time = self.model_time.max(ev.model_time);
        let rev = self.change_rev();
        let mut changed = false;
        let mut upsert = |w: &mut World, s: &Option<v1::EntitySnap>| -> Option<Eid> {
            let (eid, c) = w.upsert_snap(s.as_ref()?, rev)?;
            changed |= c;
            Some(eid)
        };
        let mut deaths = Vec::new();
        let mut place = None;
        match &ev.kind {
            Some(Kind::Birth(b)) => {
                upsert(self, &b.initiator);
                place = b.place.clone();
            }
            Some(Kind::Dead(d)) => deaths.extend(upsert(self, &d.initiator)),
            Some(Kind::UnitLost(d)) => deaths.extend(upsert(self, &d.initiator)),
            Some(Kind::Kill(k)) => {
                upsert(self, &k.initiator);
                deaths.extend(upsert(self, &k.target));
            }
            Some(Kind::UnitGone(g)) => {
                upsert(self, &g.last_known);
                deaths.extend(Eid::from_u64(g.eid));
            }
            Some(Kind::PlayerEnteredUnit(p)) => {
                if let Some(eid) = upsert(self, &p.unit)
                    && !p.player.is_empty()
                    && let Some(e) = self.entities.get(&eid)
                    && e.player.as_deref() != Some(p.player.as_str())
                {
                    let mut e = e.clone();
                    e.player = Some(p.player.clone());
                    e.rev = rev;
                    self.put_entity(e);
                    changed = true;
                }
            }
            Some(Kind::Hit(h)) => {
                upsert(self, &h.initiator);
                upsert(self, &h.target);
            }
            Some(Kind::Shot(s)) => {
                upsert(self, &s.initiator);
            }
            Some(Kind::Takeoff(t)) => {
                upsert(self, &t.initiator);
                place = t.place.clone();
            }
            Some(Kind::Land(l)) => {
                upsert(self, &l.initiator);
                place = l.place.clone();
            }
            Some(Kind::BaseCaptured(b)) => {
                upsert(self, &b.initiator);
                if let Some(a) = &b.place
                    && let Some(eid) = Eid::from_u64(a.eid)
                {
                    self.ensure_airbase(a, rev, &mut changed);
                    let owner = Coalition::from_i32(b.coalition);
                    if let Some(x) = self.airbases.get_mut(&eid)
                        && x.coalition != owner
                    {
                        x.coalition = owner;
                        x.rev = rev;
                        changed = true;
                    }
                }
            }
            _ => {}
        }
        if let Some(a) = &place {
            self.ensure_airbase(a, rev, &mut changed);
        }
        let t = self.model_time;
        for eid in deaths {
            if let Some(e) = self.entities.get_mut(&eid)
                && !e.dead
            {
                e.dead = true;
                e.died_at = Some(t);
                e.rev = rev;
                changed = true;
            }
        }
        self.commit(rev, changed);
    }

    /// Inserts an airbase carried by an event if the world does not know its Eid yet.
    fn ensure_airbase(&mut self, a: &v1::AirbaseSnap, rev: u64, changed: &mut bool) {
        if let Some(eid) = Eid::from_u64(a.eid)
            && !self.airbases.contains_key(&eid)
            && let Some(x) = Airbase::from_snap(a, rev)
        {
            self.put_airbase(x);
            *changed = true;
        }
    }
}
