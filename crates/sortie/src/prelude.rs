//! `use sortie::prelude::*;` — everything a mission usually needs.

pub use crate::{AnyGroup, Ctx, Group, Start, Task, World};
pub use serde::{Deserialize, Serialize};
pub use sortie_types::{
    Air, Alarm, AsZone, Category, Coalition, Ground, Helo, Length, MapPoint, Outcome, Roe, Ship,
    SmokeColor, Speed, StartType, Vec3, ft, hours, km, kmh, kts, m, mins, mps, nm, secs,
};
