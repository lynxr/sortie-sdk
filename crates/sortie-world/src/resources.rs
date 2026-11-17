//! The resource registry as the world shows it (`WorldSnapshot.resources`,
//! `WorldDelta.resource_upserts` / `resource_removals`).
//!
//! The engine owns the registry and writes each change here ([`World::put_resource`],
//! [`World::remove_resource`]); a replica takes them from snapshots and deltas. Resources are
//! keyed by `(kind, name)` and listed in that order. Like everything else in the world, a
//! change bumps the revision once on the mirror and a removal is logged for deltas.

use sortie_proto::v1;

use crate::{Role, World};

/// `(ResourceKind as i32, name)`: the identity of a resource.
pub(crate) type ResourceId = (i32, String);

/// One resource and the revision of its last change.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ResourceEntry {
    pub snap: v1::ResourceSnap,
    pub rev: u64,
}

pub(crate) fn id_of(snap: &v1::ResourceSnap) -> ResourceId {
    (snap.kind, snap.name.clone())
}

pub(crate) fn key_of(id: &ResourceId) -> v1::ResourceKey {
    v1::ResourceKey {
        kind: id.0,
        name: id.1.clone(),
    }
}

impl World {
    /// Every registered resource, by kind, then name.
    pub fn resources(&self) -> impl Iterator<Item = &v1::ResourceSnap> {
        self.resources.values().map(|r| &r.snap)
    }

    /// The resource `name` of `kind`, if registered.
    #[must_use]
    pub fn resource(&self, kind: v1::ResourceKind, name: &str) -> Option<&v1::ResourceSnap> {
        self.resources
            .get(&(kind as i32, name.to_owned()))
            .map(|r| &r.snap)
    }

    /// Number of registered resources.
    #[must_use]
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// Mirror side: inserts or replaces a resource. Returns whether anything changed (an
    /// equal snap changes nothing and does not bump the revision).
    pub fn put_resource(&mut self, snap: v1::ResourceSnap) -> bool {
        let rev = self.change_rev();
        let id = id_of(&snap);
        if self.resources.get(&id).is_some_and(|r| r.snap == snap) {
            return false;
        }
        self.resources.insert(id, ResourceEntry { snap, rev });
        self.commit(rev, true);
        true
    }

    /// Mirror side: removes a resource (it appears in `resource_removals` of the next delta).
    pub fn remove_resource(
        &mut self,
        kind: v1::ResourceKind,
        name: &str,
    ) -> Option<v1::ResourceSnap> {
        let rev = self.change_rev();
        let id = (kind as i32, name.to_owned());
        let r = self.resources.remove(&id)?;
        if self.role == Role::Mirror {
            self.history.resources.push((rev, id));
        }
        self.commit(rev, true);
        Some(r.snap)
    }

    /// Replica side: a resource from a command's reply. Returns whether it changed.
    pub(crate) fn observe_resource(&mut self, snap: &v1::ResourceSnap, rev: u64) -> bool {
        let id = id_of(snap);
        if self.resources.get(&id).is_some_and(|r| r.snap == *snap) {
            return false;
        }
        self.resources.insert(
            id,
            ResourceEntry {
                snap: snap.clone(),
                rev,
            },
        );
        true
    }

    /// Replica side: the resources of a snapshot.
    pub(crate) fn load_resources(&mut self, list: &[v1::ResourceSnap], rev: u64) {
        self.resources = list
            .iter()
            .map(|s| {
                (
                    id_of(s),
                    ResourceEntry {
                        snap: s.clone(),
                        rev,
                    },
                )
            })
            .collect();
    }

    /// Replica side: a delta's registry part, removals first.
    pub(crate) fn apply_resource_delta(&mut self, d: &v1::WorldDelta, rev: u64) {
        for k in &d.resource_removals {
            self.resources.remove(&(k.kind, k.name.clone()));
        }
        for s in &d.resource_upserts {
            self.resources.insert(
                id_of(s),
                ResourceEntry {
                    snap: s.clone(),
                    rev,
                },
            );
        }
    }
}
