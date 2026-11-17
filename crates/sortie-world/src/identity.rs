//! The identity table of weapons and ejected crew (engine side; spec 5.1 row "Weapon and crew
//! identity").
//!
//! Weapons and ejected crew (pilots, seats) are no mirror entities: they are never in a
//! snapshot, a delta or a query, and they do not move the revision. The engine still gives each
//! of them an [`Eid`] from the mirror's allocator, so the Eid never names anything else, and
//! keeps `runtime id -> Eid` here while DCS can still name the object in an event:
//!
//! * a weapon gets its Eid from the first event that names it (SHOT, or HIT / KILL when no SHOT
//!   came; a SHOT always starts a new identity, [`World::identify_weapon_fired`]) and keeps it
//!   until an event names the same runtime id with another type (DCS reuses `id_`), a SHOT names
//!   the runtime id again, a mirror entity takes it, the engine destroyed it, it was not named for [`WEAPON_IDLE_S`] of model time, or
//!   [`WEAPON_AFTER_STRIKE_S`] passed since the first HIT or KILL that named it;
//! * a crew member (an object of an ejection event the mirror does not know, or any nameless
//!   object the mirror does not know) keeps its Eid until its runtime id comes with another type,
//!   the engine destroyed it, or an event reported it dead. Landed pilots stay in DCS until
//!   removed, so crew Eids do not expire by time;
//! * at most [`MAX_IDENTITIES`] are kept: past that, ended entries go first, then the ones
//!   named longest ago.
//!
//! Whether DCS reuses an `id_` with the same type while the old object could still be named is
//! not measured yet (live checks EVT-L10, EVT-L16). An ended identity is never revived: a later
//! event naming the runtime id gets a fresh Eid.

use std::collections::HashMap;

use sortie_types::Eid;

use crate::World;

/// Model seconds a weapon keeps its Eid after the last event that named it.
pub const WEAPON_IDLE_S: f64 = 1800.0;
/// Model seconds a weapon keeps its Eid after the first HIT or KILL that named it (a KILL or a
/// second HIT of the same round still finds it).
pub const WEAPON_AFTER_STRIKE_S: f64 = 30.0;
/// Upper bound of the table.
pub const MAX_IDENTITIES: usize = 4096;

/// What an identity names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityKind {
    /// A weapon in flight (missile, rocket, bomb, shell).
    Weapon,
    /// An ejected crew member or the seat of one (a nameless object of class STATIC in DCS).
    Crew,
}

/// One weapon or crew member the engine named by Eid.
#[derive(Clone, Debug, PartialEq)]
pub struct Identity {
    pub eid: Eid,
    pub kind: IdentityKind,
    /// DCS `id_`.
    pub runtime_id: u32,
    /// `getTypeName()` when DCS answered; empty until it did.
    pub type_name: String,
    /// Model time of the first and the last event that named it.
    pub first_seen: f64,
    pub last_seen: f64,
    /// Weapons: model time of the first HIT or KILL that named it.
    pub struck_at: Option<f64>,
    /// Destroyed by the engine, reported dead, or its runtime id was reused.
    pub ended: bool,
}

impl Identity {
    /// Whether the Eid still names this object at model time `now`.
    #[must_use]
    pub fn is_live(&self, now: f64) -> bool {
        if self.ended {
            return false;
        }
        match self.kind {
            IdentityKind::Crew => true,
            IdentityKind::Weapon => match self.struck_at {
                Some(t) => now - t <= WEAPON_AFTER_STRIKE_S,
                None => now - self.last_seen <= WEAPON_IDLE_S,
            },
        }
    }

    fn type_matches(&self, type_name: Option<&str>) -> bool {
        match type_name {
            Some(t) if !t.is_empty() && !self.type_name.is_empty() => t == self.type_name,
            _ => true,
        }
    }
}

/// The table (a field of [`World`]).
#[derive(Clone, Debug, Default)]
pub(crate) struct Identities {
    by_eid: HashMap<Eid, Identity>,
    by_runtime: HashMap<u32, Eid>,
}

impl Identities {
    fn prune(&mut self, now: f64) {
        if self.by_eid.len() < MAX_IDENTITIES {
            return;
        }
        self.by_eid.retain(|_, i| i.is_live(now));
        let keep = MAX_IDENTITIES * 3 / 4;
        if self.by_eid.len() > keep {
            let mut ages: Vec<(f64, Eid)> =
                self.by_eid.values().map(|i| (i.last_seen, i.eid)).collect();
            ages.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let drop = self.by_eid.len() - keep;
            for (_, eid) in ages.into_iter().take(drop) {
                self.by_eid.remove(&eid);
            }
        }
        let by_eid = &self.by_eid;
        self.by_runtime.retain(|_, e| by_eid.contains_key(e));
    }
}

impl World {
    /// The Eid of the weapon with DCS runtime id `runtime_id` (see the module docs): the one it
    /// has, else a new one. `type_name` is `getTypeName()` when DCS answered; `struck` marks an
    /// event that names the weapon as the one that hit (HIT, KILL).
    pub fn identify_weapon(
        &mut self,
        runtime_id: u32,
        type_name: Option<&str>,
        struck: bool,
    ) -> Eid {
        self.identify(IdentityKind::Weapon, runtime_id, type_name, struck)
    }

    /// The Eid of a weapon just fired (SHOT): always a new one. A live identity at
    /// `runtime_id` (a weapon that vanished with no HIT, kept for [`WEAPON_IDLE_S`]) ends first,
    /// so two weapons never share an Eid.
    pub fn identify_weapon_fired(&mut self, runtime_id: u32, type_name: Option<&str>) -> Eid {
        self.end_runtime_id(runtime_id);
        self.identify(IdentityKind::Weapon, runtime_id, type_name, false)
    }

    /// Ends the identity at `runtime_id`, if any (another object holds the runtime id now).
    pub fn end_runtime_id(&mut self, runtime_id: u32) {
        if let Some(i) = self
            .identities
            .by_runtime
            .get(&runtime_id)
            .and_then(|e| self.identities.by_eid.get_mut(e))
        {
            i.ended = true;
        }
    }

    /// The Eid of the ejected crew member (or seat) with DCS runtime id `runtime_id`.
    pub fn identify_crew(&mut self, runtime_id: u32, type_name: Option<&str>) -> Eid {
        self.identify(IdentityKind::Crew, runtime_id, type_name, false)
    }

    fn identify(
        &mut self,
        kind: IdentityKind,
        runtime_id: u32,
        type_name: Option<&str>,
        struck: bool,
    ) -> Eid {
        let now = self.model_time;
        let known = self
            .identities
            .by_runtime
            .get(&runtime_id)
            .copied()
            .and_then(|e| self.identities.by_eid.get_mut(&e));
        if let Some(i) = known {
            if i.kind == kind && i.is_live(now) && i.type_matches(type_name) {
                i.last_seen = i.last_seen.max(now);
                if i.type_name.is_empty()
                    && let Some(t) = type_name
                {
                    t.clone_into(&mut i.type_name);
                }
                if struck && i.struck_at.is_none() {
                    i.struck_at = Some(now);
                }
                return i.eid;
            }
            // Another object holds the runtime id now (or this one ended).
            i.ended = true;
        }
        self.identities.prune(now);
        let eid = self.alloc_eid();
        self.identities.by_eid.insert(
            eid,
            Identity {
                eid,
                kind,
                runtime_id,
                type_name: type_name.unwrap_or_default().to_owned(),
                first_seen: now,
                last_seen: now,
                struck_at: struck.then_some(now),
                ended: false,
            },
        );
        self.identities.by_runtime.insert(runtime_id, eid);
        eid
    }

    /// The identity of a weapon or crew Eid (live or ended, while the table keeps it).
    #[must_use]
    pub fn identity(&self, eid: Eid) -> Option<&Identity> {
        self.identities.by_eid.get(&eid)
    }

    /// The live identity holding DCS runtime id `runtime_id`, if any (no mutation).
    #[must_use]
    pub fn identity_by_runtime_id(&self, runtime_id: u32) -> Option<&Identity> {
        let now = self.model_time;
        self.identities
            .by_runtime
            .get(&runtime_id)
            .and_then(|e| self.identities.by_eid.get(e))
            .filter(|i| i.is_live(now))
    }

    /// Ends an identity (the engine destroyed the object, or an event reported it dead): its
    /// Eid no longer selects anything. Returns false for an Eid that is no live identity.
    pub fn end_identity(&mut self, eid: Eid) -> bool {
        let now = self.model_time;
        match self.identities.by_eid.get_mut(&eid) {
            Some(i) if i.is_live(now) => {
                i.ended = true;
                true
            }
            _ => false,
        }
    }

    /// Number of identities the table holds (live or ended).
    #[must_use]
    pub fn identity_count(&self) -> usize {
        self.identities.by_eid.len()
    }
}
