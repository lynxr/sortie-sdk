//! [`Eid`]: Sortie's own generational entity id.
//!
//! DCS reuses runtime ids (`id_`), so every object the engine tracks gets an Eid instead.
//! Wire format (every `uint64 eid` field): `generation << 32 | index`; generation >= 1, so a
//! valid Eid is never 0 and `0` means "none".

use std::fmt;
use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

/// Generational entity id. Respawning a group under the same name yields a different Eid.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct Eid(NonZeroU64);

impl Eid {
    /// Builds an Eid from its parts; `None` if `generation` is 0.
    #[must_use]
    pub const fn from_parts(index: u32, generation: u32) -> Option<Self> {
        Self::from_u64(sortie_proto::eid::pack(index, generation))
    }

    /// Parses a wire value; `None` for 0 or a generation of 0.
    #[must_use]
    pub const fn from_u64(raw: u64) -> Option<Self> {
        if sortie_proto::eid::is_none(raw) {
            return None;
        }
        match NonZeroU64::new(raw) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Wire value.
    #[must_use]
    pub const fn to_u64(self) -> u64 {
        self.0.get()
    }

    #[must_use]
    pub const fn index(self) -> u32 {
        sortie_proto::eid::index(self.0.get())
    }

    #[must_use]
    pub const fn generation(self) -> u32 {
        sortie_proto::eid::generation(self.0.get())
    }
}

impl fmt::Debug for Eid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Eid({}v{})", self.index(), self.generation())
    }
}

impl fmt::Display for Eid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}v{}", self.index(), self.generation())
    }
}

impl From<Eid> for u64 {
    fn from(e: Eid) -> u64 {
        e.to_u64()
    }
}

/// Error for a wire value that is not a valid Eid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{0} is not a valid Eid (generation must be >= 1)")]
pub struct InvalidEid(pub u64);

impl TryFrom<u64> for Eid {
    type Error = InvalidEid;

    fn try_from(raw: u64) -> Result<Self, Self::Error> {
        Self::from_u64(raw).ok_or(InvalidEid(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_and_wire() {
        let e = Eid::from_parts(5, 1).unwrap();
        assert_eq!(e.to_u64(), (1 << 32) | 5);
        assert_eq!(e.index(), 5);
        assert_eq!(e.generation(), 1);
        assert_eq!(Eid::from_u64(0), None);
        assert_eq!(Eid::from_parts(5, 0), None);
        assert_eq!(e.to_string(), "5v1");
        assert_eq!(std::mem::size_of::<Option<Eid>>(), 8);
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(json, ((1u64 << 32) | 5).to_string());
        assert_eq!(serde_json::from_str::<Eid>(&json).unwrap(), e);
    }
}
