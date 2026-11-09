//! Vocabulary enums shared by the SDK, world, engine and fake, with lossless conversions to the
//! `sortie_proto::v1` enums (whose `*_UNSPECIFIED` value maps to `None`).

use serde::{Deserialize, Serialize};
use sortie_proto::v1;

/// A proto enum value that has no Rust counterpart (UNSPECIFIED or unknown).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{enum_name} value {value} is unspecified or unknown")]
pub struct UnknownEnumValue {
    pub enum_name: &'static str,
    pub value: i32,
}

macro_rules! proto_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident => v1::$proto:ident {
            $($(#[$vmeta:meta])* $var:ident => $pvar:ident),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($(#[$vmeta])* $var),*
        }

        impl $name {
            /// Every value, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$var),*];

            #[must_use]
            pub const fn to_proto(self) -> v1::$proto {
                match self {
                    $(Self::$var => v1::$proto::$pvar),*
                }
            }

            /// The proto wire value (as stored in `i32` enum fields).
            #[must_use]
            pub const fn as_i32(self) -> i32 {
                self.to_proto() as i32
            }

            #[must_use]
            pub const fn from_proto(p: v1::$proto) -> Option<Self> {
                match p {
                    $(v1::$proto::$pvar => Some(Self::$var),)*
                    #[allow(unreachable_patterns)]
                    _ => None,
                }
            }

            /// Parses a proto wire value; `None` for UNSPECIFIED and unknown values.
            #[must_use]
            pub fn from_i32(v: i32) -> Option<Self> {
                v1::$proto::try_from(v).ok().and_then(Self::from_proto)
            }
        }

        impl From<$name> for v1::$proto {
            fn from(v: $name) -> Self {
                v.to_proto()
            }
        }

        impl TryFrom<v1::$proto> for $name {
            type Error = UnknownEnumValue;

            fn try_from(p: v1::$proto) -> Result<Self, Self::Error> {
                Self::from_proto(p).ok_or(UnknownEnumValue { enum_name: stringify!($proto), value: p as i32 })
            }
        }
    };
}

proto_enum! {
    /// DCS coalition (side).
    pub enum Coalition => v1::Coalition {
        Neutral => Neutral,
        Red => Red,
        Blue => Blue,
    }
}

impl Coalition {
    /// The opposing side; neutral has none.
    #[must_use]
    pub const fn enemy(self) -> Option<Coalition> {
        match self {
            Coalition::Red => Some(Coalition::Blue),
            Coalition::Blue => Some(Coalition::Red),
            Coalition::Neutral => None,
        }
    }
}

proto_enum! {
    /// Object category (normalised from DCS Object.Category and Unit desc category).
    pub enum Category => v1::Category {
        Airplane => Airplane,
        Helicopter => Helicopter,
        GroundUnit => GroundUnit,
        Ship => Ship,
        Structure => Structure,
        Static => Static,
        Weapon => Weapon,
        Scenery => Scenery,
        Base => Base,
        Cargo => Cargo,
    }
}

impl Category {
    /// Airplanes and helicopters.
    #[must_use]
    pub const fn is_air(self) -> bool {
        matches!(self, Category::Airplane | Category::Helicopter)
    }

    /// Units (things that belong to groups): air, ground and ships.
    #[must_use]
    pub const fn is_unit(self) -> bool {
        matches!(
            self,
            Category::Airplane | Category::Helicopter | Category::GroundUnit | Category::Ship
        )
    }
}

proto_enum! {
    /// AI alarm state (ground and naval groups).
    pub enum Alarm => v1::AlarmState {
        Auto => Auto,
        Green => Green,
        Red => Red,
    }
}

proto_enum! {
    /// AI rules of engagement.
    pub enum Roe => v1::Roe {
        WeaponFree => WeaponFree,
        OpenFireWeaponFree => OpenFireWeaponFree,
        OpenFire => OpenFire,
        ReturnFire => ReturnFire,
        WeaponHold => WeaponHold,
    }
}

proto_enum! {
    /// Smoke marker colour.
    pub enum SmokeColor => v1::SmokeColor {
        Green => Green,
        Red => Red,
        White => White,
        Orange => Orange,
        Blue => Blue,
    }
}

proto_enum! {
    /// How a spawned flight starts at an airbase.
    pub enum StartType => v1::StartType {
        Runway => Runway,
        Hot => Hot,
        Cold => Cold,
        Air => Air,
    }
}

proto_enum! {
    /// How a session (turn) ended.
    pub enum Outcome => v1::Outcome {
        BlueWin => BlueWin,
        RedWin => RedWin,
        Draw => Draw,
        Aborted => Aborted,
        TimeLimit => TimeLimit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proto_round_trip() {
        for c in Coalition::ALL {
            assert_eq!(Coalition::from_i32(c.as_i32()), Some(*c));
        }
        for o in Outcome::ALL {
            assert_eq!(Outcome::try_from(o.to_proto()), Ok(*o));
        }
        assert_eq!(Coalition::from_i32(0), None);
        assert_eq!(Alarm::Red.to_proto(), v1::AlarmState::Red);
        assert_eq!(
            serde_json::to_string(&Outcome::BlueWin).unwrap(),
            "\"blue_win\""
        );
        assert!(Category::Helicopter.is_air());
        assert_eq!(Coalition::Blue.enemy(), Some(Coalition::Red));
    }
}
