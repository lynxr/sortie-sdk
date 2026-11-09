//! Entity ids on the wire: `uint64 = generation << 32 | index`, `0` = none.
//!
//! The typed wrapper is `sortie_types::Eid`; these helpers are for code that handles the raw
//! `uint64` fields of the generated messages.

/// The "no entity" value.
pub const NONE: u64 = 0;

/// Packs an index and a generation into a wire Eid.
#[must_use]
pub const fn pack(index: u32, generation: u32) -> u64 {
    ((generation as u64) << 32) | index as u64
}

/// Slot index of a wire Eid.
#[must_use]
pub const fn index(eid: u64) -> u32 {
    (eid & 0xFFFF_FFFF) as u32
}

/// Generation of a wire Eid. Valid Eids always have a generation >= 1.
#[must_use]
pub const fn generation(eid: u64) -> u32 {
    (eid >> 32) as u32
}

/// `true` for [`NONE`] and for any value with generation 0 (never allocated).
#[must_use]
pub const fn is_none(eid: u64) -> bool {
    generation(eid) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unpack() {
        let e = pack(7, 3);
        assert_eq!(index(e), 7);
        assert_eq!(generation(e), 3);
        assert_eq!(e, (3 << 32) | 7);
        assert!(is_none(NONE));
        assert!(is_none(pack(5, 0)));
        assert!(!is_none(e));
        assert_eq!(pack(u32::MAX, u32::MAX), u64::MAX);
    }
}
