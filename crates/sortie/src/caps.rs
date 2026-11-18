//! Capabilities: what the engine, its Lua layer and the running DCS build can do.
//!
//! * Names: the constants of this module for the typed operations ([`SPAWN_STATIC`],
//!   [`DRAW`], [`DESTROY_SCENERY`], ...) and, generated from the DCS API catalog, one per
//!   catalog entry under [`call`] (`caps::call::mission::LAND_GET_HEIGHT` is
//!   `"call:mission:land.getHeight"`).
//! * Fail at start, not in flight: `#[sortie::mission(api_rev = 1, requires = [caps::DRAW])]`
//!   sends them as `Attach.required_capabilities`; the engine refuses the attach
//!   (`AttachRejected.missing_capabilities`) when one is not AVAILABLE.
//! * At run time: [`crate::Ctx::capabilities`] is the table from `Start`, kept current by
//!   `CapabilityChanged` events (logics that declare `event_rev = 1`).
//!
//! States: AVAILABLE; NOT_IMPLEMENTED (declared, not built, or the Lua layer is older than the
//! engine); MISSING_IN_DCS (the DCS function is nil in this build); DISABLED (switched off by
//! the engine owner, off by default, or a probe-gated read); QUARANTINED (its exec op kind was
//! running when DCS crashed, or faulted the bridge: the engine owner releases it). `live` is
//! UNVERIFIED for every capability in this release.

use std::collections::BTreeMap;

use sortie_proto::v1;

mod generated;
pub use generated::call;

// ----- typed operations (0.1.0) -------------------------------------------------------------

pub const SPAWN_GROUP: &str = "spawn_group";
pub const DESTROY: &str = "destroy";
pub const SET_TASK: &str = "set_task";
pub const SET_OPTION: &str = "set_option";
pub const MESSAGE: &str = "message";
pub const EFFECT: &str = "effect";
pub const FLAG: &str = "flag";
/// The 0.1.0 F10 menu.
pub const MENU: &str = "menu";
/// The 0.1.0 map mark (registered as a drawing since 0.2.0).
pub const MARK: &str = "mark";
pub const END_SESSION: &str = "end_session";
pub const QUERY_DESC_BY_TYPE: &str = "query.desc_by_type";
pub const QUERY_LAND_HEIGHT: &str = "query.land_height";
pub const QUERY_FLAG_VALUE: &str = "query.flag_value";
pub const SERVER_SET_PAUSE: &str = "server.set_pause";
pub const SERVER_LOAD_MISSION: &str = "server.load_mission";
pub const SERVER_SEND_CHAT: &str = "server.send_chat";
/// Lua by string: only with `sortied --dev-eval`.
pub const DEV_EVAL: &str = "dev_eval";

// ----- typed operations (API revision 1) ----------------------------------------------------

/// The universal call as a whole (every `call:*` capability depends on it).
pub const DCS_CALL: &str = "dcs_call";
pub const DCS_BATCH: &str = "dcs_batch";
pub const SPAWN_STATIC: &str = "spawn_static";
/// Many groups and statics in one command (`ctx.spawn_batch()`), spread by the spawn gate.
pub const SPAWN_BATCH: &str = "spawn_batch";
/// Destroying scenery: NOT_IMPLEMENTED (DCS's scenery destroy is broken).
pub const DESTROY_SCENERY: &str = "destroy.scenery";
/// Destroying a weapon in flight or an ejected crew member by its Eid (`ctx.destroy(eid)`).
pub const DESTROY_WEAPON: &str = "destroy.weapon";
pub const REMOVE_JUNK: &str = "remove_junk";
/// Objects DCS finds inside a volume (`ctx.search_objects(..)`, API revision 2).
pub const SEARCH_OBJECTS: &str = "search_objects";
pub const DRAW: &str = "draw";
/// F10 menu trees (`ctx.menus().tree(..)`).
pub const MENU_TREE: &str = "menu_tree";

/// Every typed capability name above, in the engine's order.
pub const TYPED: &[&str] = &[
    SPAWN_GROUP,
    DESTROY,
    SET_TASK,
    SET_OPTION,
    MESSAGE,
    EFFECT,
    FLAG,
    MENU,
    MARK,
    END_SESSION,
    QUERY_DESC_BY_TYPE,
    QUERY_LAND_HEIGHT,
    QUERY_FLAG_VALUE,
    SERVER_SET_PAUSE,
    SERVER_LOAD_MISSION,
    SERVER_SEND_CHAT,
    DEV_EVAL,
    DCS_CALL,
    DCS_BATCH,
    SPAWN_STATIC,
    SPAWN_BATCH,
    DESTROY_SCENERY,
    DESTROY_WEAPON,
    REMOVE_JUNK,
    SEARCH_OBJECTS,
    DRAW,
    MENU_TREE,
];

/// The capability table of a session (`Start.capabilities`, then `CapabilityChanged`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Capabilities {
    order: Vec<String>,
    by_name: BTreeMap<String, v1::Capability>,
}

impl Capabilities {
    /// The table the engine sent (`Start.capabilities`, `CapabilitiesResponse.capabilities`).
    #[must_use]
    pub fn from_proto(caps: &[v1::Capability]) -> Self {
        let mut t = Self::default();
        for c in caps {
            t.put(c.clone());
        }
        t
    }

    /// Replaces (or adds) one entry, as a `CapabilityChanged` does.
    pub fn put(&mut self, c: v1::Capability) {
        if !self.by_name.contains_key(&c.name) {
            self.order.push(c.name.clone());
        }
        self.by_name.insert(c.name.clone(), c);
    }

    /// The entry named `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&v1::Capability> {
        self.by_name.get(name)
    }

    /// Its state; NOT_IMPLEMENTED for a name the engine does not list (a 0.1.0 engine lists
    /// nothing).
    #[must_use]
    pub fn state(&self, name: &str) -> v1::CapabilityState {
        self.get(name)
            .and_then(|c| v1::CapabilityState::try_from(c.state).ok())
            .unwrap_or(v1::CapabilityState::NotImplemented)
    }

    /// `true` when `name` is AVAILABLE.
    #[must_use]
    pub fn is_available(&self, name: &str) -> bool {
        self.state(name) == v1::CapabilityState::Available
    }

    /// Why `name` is not available (the engine's note), empty when it is.
    #[must_use]
    pub fn note(&self, name: &str) -> &str {
        self.get(name).map_or("", |c| c.note.as_str())
    }

    /// Every entry, in the engine's order.
    pub fn iter(&self) -> impl Iterator<Item = &v1::Capability> {
        self.order.iter().filter_map(|n| self.by_name.get(n))
    }

    /// The entries whose state is `state`.
    pub fn in_state(&self, state: v1::CapabilityState) -> impl Iterator<Item = &v1::Capability> {
        self.iter().filter(move |c| c.state == state as i32)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.order.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(name: &str, state: v1::CapabilityState) -> v1::Capability {
        v1::Capability {
            name: name.into(),
            state: state as i32,
            ..Default::default()
        }
    }

    #[test]
    fn lookups_and_changes() {
        let mut t = Capabilities::from_proto(&[
            cap(DRAW, v1::CapabilityState::Available),
            cap(DESTROY_SCENERY, v1::CapabilityState::NotImplemented),
        ]);
        assert!(t.is_available(DRAW));
        assert!(!t.is_available(DESTROY_SCENERY));
        assert_eq!(t.state("nope"), v1::CapabilityState::NotImplemented);
        t.put(cap(DRAW, v1::CapabilityState::Quarantined));
        assert_eq!(t.state(DRAW), v1::CapabilityState::Quarantined);
        assert_eq!(t.len(), 2, "replaced in place");
        let names: Vec<&str> = t.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, [DRAW, DESTROY_SCENERY]);
        assert_eq!(t.in_state(v1::CapabilityState::Quarantined).count(), 1);
        assert_eq!(
            call::mission::LAND_GET_HEIGHT,
            "call:mission:land.getHeight"
        );
    }
}
