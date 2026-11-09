//! Typed scenario handles. Generated code declares them as constants, e.g.
//!
//! ```
//! # use sortie_types::*;
//! # #[derive(Clone, Copy, Debug, Default)] struct OpNorth;
//! const SAM_NORTH: GroupRef<OpNorth, Ground> = GroupRef::new("SAM_NORTH");
//! const SAM_RING: ZoneRef<OpNorth> = ZoneRef::circle("SAM_RING", MapPoint::new(-32798.0, 463776.3), 60_000.0);
//! assert_eq!(SAM_NORTH.ident(), "SAM_NORTH");
//! assert_eq!(SAM_RING.center(), MapPoint::new(-32798.0, 463776.3));
//! ```
//!
//! The scenario marker `M` makes handles of one scenario unusable in `Ctx` of another; `K` is
//! the group kind. Handles carry only the symbol (and, for zones, the geometry from the lock
//! file); DCS names and live geometry come from the manifest at run time.

use std::borrow::Cow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::geo::{AsZone, MapPoint, ZoneShape};
use crate::kind::Kind;

/// `fn() -> T` keeps handles `Send + Sync + Copy` whatever the marker types are.
type Marker<T> = PhantomData<fn() -> T>;

macro_rules! ident_handle {
    ($(#[$doc:meta])* $name:ident<$($p:ident),+>) => {
        $(#[$doc])*
        pub struct $name<$($p),+> {
            ident: &'static str,
            _marker: Marker<($($p,)+)>,
        }

        impl<$($p),+> $name<$($p),+> {
            #[must_use]
            pub const fn new(ident: &'static str) -> Self {
                Self { ident, _marker: PhantomData }
            }

            /// The manifest symbol, e.g. `"SAM_NORTH"`.
            #[must_use]
            pub const fn ident(&self) -> &'static str {
                self.ident
            }
        }

        impl<$($p),+> Clone for $name<$($p),+> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<$($p),+> Copy for $name<$($p),+> {}

        impl<$($p),+> PartialEq for $name<$($p),+> {
            fn eq(&self, other: &Self) -> bool {
                self.ident == other.ident
            }
        }

        impl<$($p),+> Eq for $name<$($p),+> {}

        impl<$($p),+> Hash for $name<$($p),+> {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.ident.hash(state);
            }
        }

        impl<$($p),+> fmt::Debug for $name<$($p),+> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self.ident)
            }
        }
    };
}

ident_handle! {
    /// A group placed in the mission, of kind `K`.
    GroupRef<M, K>
}

ident_handle! {
    /// A spawnable template (kept out of the .miz), of kind `K`.
    TemplateRef<M, K>
}

ident_handle! {
    /// A user flag.
    FlagRef<M>
}

ident_handle! {
    /// An airbase.
    AirbaseRef<M>
}

ident_handle! {
    /// An F10 radio menu command.
    MenuRef<M>
}

impl<M, K: Kind> GroupRef<M, K> {
    /// Lock-file kind name of this handle's group ("ground", "air", ...).
    #[must_use]
    pub const fn kind_name(&self) -> &'static str {
        K::NAME
    }
}

impl<M, K: Kind> TemplateRef<M, K> {
    /// Spawn under an explicit, unique group name instead of `<template>#<session4>-<n>`.
    #[must_use]
    pub fn named(self, name: impl Into<Cow<'static, str>>) -> TemplateSpawn<M, K> {
        TemplateSpawn {
            template: self,
            name: Some(name.into()),
        }
    }
}

/// What to spawn: a template and an optional explicit group name.
pub struct TemplateSpawn<M, K> {
    pub template: TemplateRef<M, K>,
    pub name: Option<Cow<'static, str>>,
}

impl<M, K> From<TemplateRef<M, K>> for TemplateSpawn<M, K> {
    fn from(template: TemplateRef<M, K>) -> Self {
        Self {
            template,
            name: None,
        }
    }
}

impl<M, K> Clone for TemplateSpawn<M, K> {
    fn clone(&self) -> Self {
        Self {
            template: self.template,
            name: self.name.clone(),
        }
    }
}

impl<M, K> fmt::Debug for TemplateSpawn<M, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TemplateSpawn")
            .field("template", &self.template)
            .field("name", &self.name)
            .finish()
    }
}

/// A trigger zone with the geometry recorded in scenario.lock.json.
pub struct ZoneRef<M> {
    ident: &'static str,
    shape: ZoneShape<'static>,
    _marker: Marker<M>,
}

impl<M> ZoneRef<M> {
    #[must_use]
    pub const fn circle(ident: &'static str, center: MapPoint, radius_m: f64) -> Self {
        Self {
            ident,
            shape: ZoneShape::Circle { center, radius_m },
            _marker: PhantomData,
        }
    }

    #[must_use]
    pub const fn polygon(ident: &'static str, points: &'static [MapPoint]) -> Self {
        Self {
            ident,
            shape: ZoneShape::Polygon { points },
            _marker: PhantomData,
        }
    }

    #[must_use]
    pub const fn ident(&self) -> &'static str {
        self.ident
    }

    /// Geometry from the lock file (the manifest of the running mission takes precedence in
    /// world queries).
    #[must_use]
    pub const fn shape(&self) -> ZoneShape<'static> {
        self.shape
    }

    /// Circle centre or polygon vertex average.
    #[must_use]
    pub fn center(&self) -> MapPoint {
        self.shape.center()
    }

    #[must_use]
    pub fn contains(&self, p: MapPoint) -> bool {
        self.shape.contains(p)
    }
}

impl<M> AsZone for ZoneRef<M> {
    fn zone_ident(&self) -> &str {
        self.ident
    }

    fn zone_shape(&self) -> ZoneShape<'_> {
        self.shape
    }

    fn zone_shape_static(&self) -> Option<ZoneShape<'static>> {
        Some(self.shape)
    }
}

impl<M> Clone for ZoneRef<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for ZoneRef<M> {}

impl<M> PartialEq for ZoneRef<M> {
    fn eq(&self, other: &Self) -> bool {
        self.ident == other.ident
    }
}

impl<M> fmt::Debug for ZoneRef<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ZoneRef")
            .field("ident", &self.ident)
            .field("shape", &self.shape)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kind::{Air, Ground};

    #[derive(Clone, Copy, Debug, Default)]
    struct OpNorth;

    const SAM: GroupRef<OpNorth, Ground> = GroupRef::new("SAM_NORTH");
    const CAP: TemplateRef<OpNorth, Air> = TemplateRef::new("RED_CAP");

    fn assert_send_sync<T: Send + Sync + Copy>(_: T) {}

    #[test]
    fn handles() {
        assert_send_sync(SAM);
        assert_eq!(SAM.kind_name(), "ground");
        let spawn = CAP.named("Red CAP 1");
        assert_eq!(spawn.name.as_deref(), Some("Red CAP 1"));
        assert_eq!(spawn.template.ident(), "RED_CAP");
        assert_eq!(format!("{SAM:?}"), "GroupRef(SAM_NORTH)");
    }
}
