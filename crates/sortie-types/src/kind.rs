//! Kind markers for typed group handles: `GroupRef<OpNorth, Ground>`, `Group<Air>`, ...
//!
//! Lock files spell kinds as `"ground" | "air" | "helo" | "ship"` ([`Kind::NAME`]).

use std::fmt::Debug;

use crate::vocab::Category;

mod sealed {
    pub trait Sealed {}
}

/// A group kind. Sealed: exactly [`Ground`], [`Air`], [`Helo`], [`Ship`].
pub trait Kind: sealed::Sealed + Copy + Default + Debug + Send + Sync + 'static {
    /// Spelling in scenario.lock.json.
    const NAME: &'static str;
    /// Manifest / proto category of groups of this kind.
    const CATEGORY: Category;
}

/// Kinds that fly (can take CAP/orbit/RTB tasks).
pub trait Flying: Kind {}

macro_rules! kinds {
    ($($(#[$doc:meta])* $ty:ident = $name:literal, $cat:ident;)*) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub struct $ty;
        impl sealed::Sealed for $ty {}
        impl Kind for $ty {
            const NAME: &'static str = $name;
            const CATEGORY: Category = Category::$cat;
        }
    )*};
}

kinds! {
    /// Ground units (vehicles, SAM sites).
    Ground = "ground", GroundUnit;
    /// Fixed-wing aircraft.
    Air = "air", Airplane;
    /// Helicopters.
    Helo = "helo", Helicopter;
    /// Ships.
    Ship = "ship", Ship;
}

impl Flying for Air {}
impl Flying for Helo {}

/// Parses a lock-file kind name.
#[must_use]
pub fn category_of(name: &str) -> Option<Category> {
    match name {
        Ground::NAME => Some(Ground::CATEGORY),
        Air::NAME => Some(Air::CATEGORY),
        Helo::NAME => Some(Helo::CATEGORY),
        Ship::NAME => Some(Ship::CATEGORY),
        _ => None,
    }
}
