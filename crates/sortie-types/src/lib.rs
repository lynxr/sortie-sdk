//! The shared vocabulary of Sortie.
//!
//! This small crate is the single home of the types that generated scenario code
//! (`sortie-build`), the SDK (`sortie`), the world model (`sortie-world`), the engine and the
//! fake all agree on:
//!
//! * typed scenario handles — [`GroupRef`], [`ZoneRef`], [`TemplateRef`], [`FlagRef`],
//!   [`AirbaseRef`], [`MenuRef`] — and the [`Scenario`] trait implemented by generated
//!   scenario markers such as `OpNorth`;
//! * kind markers [`kind::Ground`], [`kind::Air`], [`kind::Helo`], [`kind::Ship`];
//! * geometry in DCS axes ([`MapPoint`], [`Vec3`], [`ZoneShape`]) and units ([`Length`],
//!   [`Speed`], [`ft`], [`kts`], [`secs`], ...);
//! * vocabulary enums ([`Coalition`], [`Category`], [`Alarm`], [`Roe`], [`SmokeColor`],
//!   [`StartType`], [`Outcome`]) with conversions to and from `sortie_proto::v1`;
//! * [`Eid`], the generational entity id;
//! * [`Milestone`] / [`NotYet`] for friendly "lands in Mx" errors.
//!
//! Generated code refers to these types as `::sortie::types::*` (the SDK re-exports this crate),
//! so user crates only depend on `sortie`.

pub mod eid;
pub mod geo;
pub mod handles;
pub mod kind;
pub mod milestone;
pub mod scenario;
pub mod units;
pub mod vocab;

pub use eid::Eid;
pub use geo::{AsZone, Bounds, MapPoint, Vec3, ZoneShape};
pub use handles::{AirbaseRef, FlagRef, GroupRef, MenuRef, TemplateRef, TemplateSpawn, ZoneRef};
pub use kind::{Air, Ground, Helo, Kind, Ship};
pub use milestone::{Milestone, NotYet};
pub use scenario::{Scenario, SchemaHashError, Symbol, SymbolKind, schema_sha256_bytes};
pub use units::{Length, Speed, ft, hours, km, kmh, kts, m, mins, mps, nm, secs};
pub use vocab::{
    Alarm, Category, Coalition, Outcome, Roe, SmokeColor, StartType, UnknownEnumValue,
};
