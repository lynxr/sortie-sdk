//! Units: [`Length`] (metres inside), [`Speed`] (m/s inside) and `Duration` helpers.
//!
//! ```
//! use sortie_types::{ft, kts, secs};
//! assert!((ft(25_000).meters() - 7620.0).abs() < 1e-9);
//! assert!((kts(420).mps() - 216.0666).abs() < 1e-3);
//! assert_eq!(secs(90).as_secs(), 90);
//! ```

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::time::Duration;

use serde::{Deserialize, Serialize};

const FOOT_M: f64 = 0.3048;
const NAUTICAL_MILE_M: f64 = 1852.0;
const KNOT_MPS: f64 = NAUTICAL_MILE_M / 3600.0;

/// A distance or altitude.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Length(f64);

impl Length {
    #[must_use]
    pub const fn from_meters(meters: f64) -> Self {
        Self(meters)
    }

    #[must_use]
    pub const fn meters(self) -> f64 {
        self.0
    }

    #[must_use]
    pub fn feet(self) -> f64 {
        self.0 / FOOT_M
    }

    #[must_use]
    pub fn nautical_miles(self) -> f64 {
        self.0 / NAUTICAL_MILE_M
    }

    #[must_use]
    pub fn kilometers(self) -> f64 {
        self.0 / 1000.0
    }
}

macro_rules! scalar_ops {
    ($ty:ident) => {
        impl Add for $ty {
            type Output = $ty;
            fn add(self, o: $ty) -> $ty {
                $ty(self.0 + o.0)
            }
        }

        impl Sub for $ty {
            type Output = $ty;
            fn sub(self, o: $ty) -> $ty {
                $ty(self.0 - o.0)
            }
        }

        impl Mul<f64> for $ty {
            type Output = $ty;
            fn mul(self, k: f64) -> $ty {
                $ty(self.0 * k)
            }
        }

        impl Div<f64> for $ty {
            type Output = $ty;
            fn div(self, k: f64) -> $ty {
                $ty(self.0 / k)
            }
        }

        impl Neg for $ty {
            type Output = $ty;
            fn neg(self) -> $ty {
                $ty(-self.0)
            }
        }
    };
}

scalar_ops!(Length);
scalar_ops!(Speed);

impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} m", self.0)
    }
}

impl fmt::Display for Speed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} m/s", self.0)
    }
}

/// A speed.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Speed(f64);

impl Speed {
    #[must_use]
    pub const fn from_mps(mps: f64) -> Self {
        Self(mps)
    }

    #[must_use]
    pub const fn mps(self) -> f64 {
        self.0
    }

    #[must_use]
    pub fn knots(self) -> f64 {
        self.0 / KNOT_MPS
    }

    #[must_use]
    pub fn kmh(self) -> f64 {
        self.0 * 3.6
    }
}

/// Metres.
pub fn m(v: impl Into<f64>) -> Length {
    Length(v.into())
}

/// Kilometres.
pub fn km(v: impl Into<f64>) -> Length {
    Length(v.into() * 1000.0)
}

/// Feet.
pub fn ft(v: impl Into<f64>) -> Length {
    Length(v.into() * FOOT_M)
}

/// Nautical miles.
pub fn nm(v: impl Into<f64>) -> Length {
    Length(v.into() * NAUTICAL_MILE_M)
}

/// Knots.
pub fn kts(v: impl Into<f64>) -> Speed {
    Speed(v.into() * KNOT_MPS)
}

/// Metres per second.
pub fn mps(v: impl Into<f64>) -> Speed {
    Speed(v.into())
}

/// Kilometres per hour.
pub fn kmh(v: impl Into<f64>) -> Speed {
    Speed(v.into() / 3.6)
}

/// Seconds (of model time when passed to `ctx.sleep`, `ctx.timer`, `FakeDcs::advance`).
#[must_use]
pub const fn secs(v: u64) -> Duration {
    Duration::from_secs(v)
}

/// Minutes.
#[must_use]
pub const fn mins(v: u64) -> Duration {
    Duration::from_secs(v * 60)
}

/// Hours.
#[must_use]
pub const fn hours(v: u64) -> Duration {
    Duration::from_secs(v * 3600)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_and_ops() {
        assert!((nm(1).meters() - 1852.0).abs() < 1e-9);
        assert!((km(2).kilometers() - 2.0).abs() < 1e-12);
        assert!((ft(1000).feet() - 1000.0).abs() < 1e-9);
        assert!((kmh(36).mps() - 10.0).abs() < 1e-12);
        assert!((mps(10).kmh() - 36.0).abs() < 1e-12);
        assert!((kts(1).knots() - 1.0).abs() < 1e-12);
        assert_eq!(m(3) + m(4), m(7));
        assert_eq!(m(10) - m(4), m(6));
        assert_eq!(m(10) * 2.0, m(20));
        assert_eq!(m(10) / 2.0, m(5));
        assert_eq!(-m(1), m(-1));
        assert!(km(1) > m(999));
        assert_eq!(m(5).to_string(), "5 m");
        assert_eq!(mins(2), secs(120));
        assert_eq!(hours(1), mins(60));
    }
}
