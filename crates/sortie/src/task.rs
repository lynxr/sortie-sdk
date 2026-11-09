//! AI tasks for [`crate::Group::push_task`]. The engine's codec turns them into DCS task tables.

use sortie_proto::{LuaValue, v1};
use sortie_types::{AirbaseRef, AsZone, Length, MapPoint, Speed, kts, m};

/// A task for an air group.
#[derive(Clone, Debug, PartialEq)]
pub enum Task {
    Cap(CapTask),
    Orbit(OrbitTask),
    Rtb {
        airbase: String,
    },
    /// A raw DCS task table (escape hatch; not checked).
    Raw(LuaValue),
}

/// Combat air patrol over a zone.
#[derive(Clone, Debug, PartialEq)]
pub struct CapTask {
    pub zone: String,
    pub altitude: Length,
    pub speed: Speed,
}

/// Orbit over a point.
#[derive(Clone, Debug, PartialEq)]
pub struct OrbitTask {
    pub center: MapPoint,
    pub altitude: Length,
    pub speed: Speed,
    /// Second point: race track; none: circle.
    pub second: Option<MapPoint>,
}

impl Task {
    /// CAP over `zone` (defaults: 6000 m, 420 kt; set with `.altitude()` / `.speed()`).
    pub fn cap(zone: impl AsZone) -> CapTask {
        CapTask {
            zone: zone.zone_ident().to_owned(),
            altitude: m(6000),
            speed: kts(420),
        }
    }

    #[must_use]
    pub fn orbit(center: MapPoint) -> OrbitTask {
        OrbitTask {
            center,
            altitude: m(6000),
            speed: kts(350),
            second: None,
        }
    }

    #[must_use]
    pub fn rtb<M>(airbase: AirbaseRef<M>) -> Task {
        Task::Rtb {
            airbase: airbase.ident().to_owned(),
        }
    }

    #[must_use]
    pub fn raw(task: LuaValue) -> Task {
        Task::Raw(task)
    }

    pub(crate) fn to_proto(&self) -> v1::set_task::Task {
        match self {
            Task::Cap(c) => v1::set_task::Task::Cap(v1::CapTask {
                zone: c.zone.clone(),
                altitude_m: c.altitude.meters(),
                speed_mps: c.speed.mps(),
            }),
            Task::Orbit(o) => v1::set_task::Task::Orbit(v1::OrbitTask {
                center: Some(o.center.into()),
                altitude_m: o.altitude.meters(),
                speed_mps: o.speed.mps(),
                pattern: if o.second.is_some() {
                    v1::OrbitPattern::RaceTrack
                } else {
                    v1::OrbitPattern::Circle
                } as i32,
                second: o.second.map(Into::into),
            }),
            Task::Rtb { airbase } => v1::set_task::Task::Rtb(v1::RtbTask {
                airbase: airbase.clone(),
            }),
            Task::Raw(v) => v1::set_task::Task::Raw(v.clone()),
        }
    }
}

impl CapTask {
    #[must_use]
    pub fn altitude(mut self, altitude: Length) -> Self {
        self.altitude = altitude;
        self
    }

    #[must_use]
    pub fn speed(mut self, speed: Speed) -> Self {
        self.speed = speed;
        self
    }
}

impl OrbitTask {
    #[must_use]
    pub fn altitude(mut self, altitude: Length) -> Self {
        self.altitude = altitude;
        self
    }

    #[must_use]
    pub fn speed(mut self, speed: Speed) -> Self {
        self.speed = speed;
        self
    }

    #[must_use]
    pub fn race_track(mut self, second: MapPoint) -> Self {
        self.second = Some(second);
        self
    }
}

impl From<CapTask> for Task {
    fn from(t: CapTask) -> Self {
        Task::Cap(t)
    }
}

impl From<OrbitTask> for Task {
    fn from(t: OrbitTask) -> Self {
        Task::Orbit(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sortie_types::{ZoneRef, ft};

    #[test]
    fn cap_task_to_proto() {
        const RING: ZoneRef<()> = ZoneRef::circle("SAM_RING", MapPoint::new(0.0, 0.0), 1.0);
        let t: Task = Task::cap(RING).altitude(ft(25_000)).speed(kts(420)).into();
        let v1::set_task::Task::Cap(c) = t.to_proto() else {
            panic!()
        };
        assert_eq!(c.zone, "SAM_RING");
        assert!((c.altitude_m - 7620.0).abs() < 1e-9);
        assert!((c.speed_mps - 216.067).abs() < 1e-2);
    }
}
