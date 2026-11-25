//! AI tasks for [`crate::Group::push_task`] and [`crate::Group::set_task`]. The engine's codec
//! turns them into DCS task tables. Every group takes a raw DCS task table ([`Task::raw`]); CAP,
//! orbit and RTB are for aircraft and helicopters, a route ([`Task::ground_route`]) for ground
//! groups ([`TaskFor`]).

use sortie_proto::{LuaValue, v1};
use sortie_types::kind::Flying;
use sortie_types::{AirbaseRef, AsZone, Ground, Kind, Length, MapPoint, Speed, kts, m};

/// A task for a group. `Cap`, `Orbit` and `Rtb` are for aircraft and helicopters; `GroundRoute`
/// (API revision 3) is for ground groups; a raw DCS task table ([`Task::raw`]) is for any group,
/// and the only task of a naval group. The engine refuses a task for the wrong kind of group
/// with `Error::Invalid` before anything reaches DCS.
#[derive(Clone, Debug, PartialEq)]
pub enum Task {
    Cap(CapTask),
    Orbit(OrbitTask),
    Rtb {
        airbase: String,
    },
    /// The route of a ground group ([`Task::ground_route`], API revision 3).
    GroundRoute(GroundRoute),
    /// A raw DCS task table (escape hatch: the engine checks it for Lua code, file names and
    /// non-finite numbers only, `sortie::rejected` / `sortie::invalid`).
    Raw(LuaValue),
}

/// What [`crate::Group::push_task`] and [`crate::Group::set_task`] of a group of kind `K` take:
/// a [`Task`] for every kind, a [`CapTask`] or an [`OrbitTask`] only for a flying kind (`Air`,
/// `Helo`), a [`GroundRoute`] only for `Ground`. A `Task` of the wrong kind compiles (it is one
/// enum), and the engine refuses it: a ground group takes a route or [`Task::raw`], a naval
/// group only [`Task::raw`].
///
/// ```
/// use sortie::prelude::*;
/// use sortie::proto::{LuaTable, LuaValue};
///
/// fn hold(convoy: &Group<Ground>) -> sortie::Cmd<sortie::cmd::SetTask> {
///     convoy.set_task(Task::raw(LuaValue::table(LuaTable::new().with("id", "Hold"))))
/// }
///
/// fn drive(convoy: &Group<Ground>, to: MapPoint) -> sortie::Cmd<sortie::cmd::SetTask> {
///     convoy.set_task(Task::ground_route([RoutePoint::on_road(to)]))
/// }
/// ```
///
/// An aircraft task for a ground group does not compile ("`Ground` groups do not fly"):
///
/// ```compile_fail,E0277
/// use sortie::prelude::*;
///
/// fn patrol(convoy: &Group<Ground>, at: MapPoint) -> sortie::Cmd<sortie::cmd::SetTask> {
///     convoy.set_task(Task::orbit(at))
/// }
/// ```
///
/// Nor does a ground route for a ship or an aircraft:
///
/// ```compile_fail,E0277
/// use sortie::prelude::*;
///
/// fn sail(ship: &Group<Ship>, to: MapPoint) -> sortie::Cmd<sortie::cmd::SetTask> {
///     ship.set_task(Task::ground_route([RoutePoint::off_road(to)]))
/// }
/// ```
///
/// ```compile_fail,E0277
/// use sortie::prelude::*;
///
/// fn taxi(jet: &Group<Air>, to: MapPoint) -> sortie::Cmd<sortie::cmd::SetTask> {
///     jet.push_task(Task::ground_route([RoutePoint::on_road(to)]))
/// }
/// ```
///
/// Sealed: the tasks are this module's types. In 0.3.0 the argument was `impl Into<Task>`, so
/// a helper that passes on a generic `impl Into<Task>` (or `T: Into<Task>`) no longer compiles:
///
/// ```compile_fail,E0277
/// use sortie::prelude::*;
/// use sortie::types::kind::Flying;
///
/// fn give<K: Flying>(g: &Group<K>, task: impl Into<Task>) -> sortie::Cmd<sortie::cmd::SetTask> {
///     g.push_task(task)
/// }
/// ```
///
/// It converts the task to a [`Task`] first:
///
/// ```
/// use sortie::prelude::*;
/// use sortie::types::kind::Flying;
///
/// fn give<K: Flying>(g: &Group<K>, task: impl Into<Task>) -> sortie::Cmd<sortie::cmd::SetTask> {
///     g.push_task(task.into())
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a task for a `Group<{K}>`",
    label = "not a task",
    note = "a group takes a `Task` (`Task::raw(..)` for any group); `Task::cap(..)` and \
            `Task::orbit(..)` are for aircraft and helicopters, `Task::ground_route(..)` for \
            ground groups"
)]
pub trait TaskFor<K: Kind>: Into<Task> + sealed::Sealed {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Task {}
    impl Sealed for super::CapTask {}
    impl Sealed for super::OrbitTask {}
    impl Sealed for super::GroundRoute {}
}

impl<K: Kind> TaskFor<K> for Task {}
impl<K: Flying> TaskFor<K> for CapTask {}
impl<K: Flying> TaskFor<K> for OrbitTask {}
impl TaskFor<Ground> for GroundRoute {}

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

/// The route of a ground group ([`Task::ground_route`]; API revision 3, the logic declares
/// `#[sortie::mission(api_rev = 3)]`). The engine builds the DCS `Mission` task the Mission
/// Editor writes for vehicles: a first point where the group's first live unit stands when DCS
/// takes the task, with the formation and speed of the first point, then [`Self::points`] in
/// order. A point's formation and speed are those of the leg that ends at it.
#[derive(Clone, Debug, PartialEq)]
pub struct GroundRoute {
    /// 1 to 256 points; the engine refuses an empty or a longer route (`sortie::invalid`).
    pub points: Vec<RoutePoint>,
    /// The speed of every point that sets none; `None`: 20 km/h (the Mission Editor's default
    /// for a vehicle).
    pub speed: Option<Speed>,
}

/// One point of a [`GroundRoute`]: where the group drives, how and how fast. Build it with
/// [`RoutePoint::on_road`], [`RoutePoint::off_road`], [`RoutePoint::on_railroads`] or
/// [`RoutePoint::new`] with a [`GroundFormation`]; [`RoutePoint::speed`] sets its speed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoutePoint {
    /// Map metres; a zone's centre, a point of a road path, `Vec3::map_point()` of a unit.
    pub point: MapPoint,
    /// How the group drives to this point.
    pub formation: GroundFormation,
    /// `None`: the route's speed.
    pub speed: Option<Speed>,
}

/// How a ground group drives a leg of its route: DCS's waypoint actions for vehicles (the
/// engine maps each to DCS's own name).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GroundFormation {
    /// Straight across the ground, in column ("Off Road").
    #[default]
    OffRoad,
    /// Along the roads ("On Road").
    OnRoad,
    /// Along the railways, for trains ("On Railroads").
    OnRailroads,
    /// Off road in line abreast ("Rank").
    Rank,
    /// Off road in a cone ("Cone").
    Cone,
    /// Off road in a vee ("Vee").
    Vee,
    /// Off road in a diamond ("Diamond").
    Diamond,
    /// Off road in echelon to the left ("EchelonL").
    EchelonLeft,
    /// Off road in echelon to the right ("EchelonR").
    EchelonRight,
}

impl GroundFormation {
    fn to_proto(self) -> v1::GroundFormation {
        match self {
            GroundFormation::OffRoad => v1::GroundFormation::OffRoad,
            GroundFormation::OnRoad => v1::GroundFormation::OnRoad,
            GroundFormation::OnRailroads => v1::GroundFormation::OnRailroads,
            GroundFormation::Rank => v1::GroundFormation::Rank,
            GroundFormation::Cone => v1::GroundFormation::Cone,
            GroundFormation::Vee => v1::GroundFormation::Vee,
            GroundFormation::Diamond => v1::GroundFormation::Diamond,
            GroundFormation::EchelonLeft => v1::GroundFormation::EchelonLeft,
            GroundFormation::EchelonRight => v1::GroundFormation::EchelonRight,
        }
    }
}

impl RoutePoint {
    /// A point the group drives to in `formation`.
    #[must_use]
    pub const fn new(point: MapPoint, formation: GroundFormation) -> Self {
        Self {
            point,
            formation,
            speed: None,
        }
    }

    /// A point the group drives to along the roads.
    #[must_use]
    pub const fn on_road(point: MapPoint) -> Self {
        Self::new(point, GroundFormation::OnRoad)
    }

    /// A point the group drives to straight across the ground, in column.
    #[must_use]
    pub const fn off_road(point: MapPoint) -> Self {
        Self::new(point, GroundFormation::OffRoad)
    }

    /// A point a train drives to along the railways.
    #[must_use]
    pub const fn on_railroads(point: MapPoint) -> Self {
        Self::new(point, GroundFormation::OnRailroads)
    }

    /// The speed of the leg that ends at this point.
    #[must_use]
    pub const fn speed(mut self, speed: Speed) -> Self {
        self.speed = Some(speed);
        self
    }

    fn to_proto(self) -> v1::GroundRoutePoint {
        v1::GroundRoutePoint {
            point: Some(self.point.into()),
            formation: self.formation.to_proto() as i32,
            speed_mps: self.speed.map(Speed::mps),
        }
    }
}

impl GroundRoute {
    /// The speed of every point that sets none (default 20 km/h).
    #[must_use]
    pub fn speed(mut self, speed: Speed) -> Self {
        self.speed = Some(speed);
        self
    }
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

    /// The route of a ground group through `points` (API revision 3: the logic declares
    /// `#[sortie::mission(api_rev = 3)]`; below that, or against an older engine, the command
    /// fails with `Error::Revision` / `Error::EngineTooOld` and nothing is sent). Only a
    /// `Group<Ground>` takes it ([`TaskFor`]). The group starts where its first live unit stands.
    ///
    /// ```
    /// use sortie::prelude::*;
    ///
    /// fn convoy_route(depot: MapPoint, front: MapPoint) -> sortie::task::GroundRoute {
    ///     Task::ground_route([
    ///         RoutePoint::on_road(depot),
    ///         RoutePoint::new(front, GroundFormation::Vee).speed(kmh(15)),
    ///     ])
    ///     .speed(kmh(40))
    /// }
    /// ```
    #[must_use]
    pub fn ground_route(points: impl IntoIterator<Item = RoutePoint>) -> GroundRoute {
        GroundRoute {
            points: points.into_iter().collect(),
            speed: None,
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
            Task::GroundRoute(r) => v1::set_task::Task::GroundRoute(v1::GroundRoute {
                points: r.points.iter().map(|p| p.to_proto()).collect(),
                speed_mps: r.speed.map(Speed::mps),
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

impl From<GroundRoute> for Task {
    fn from(t: GroundRoute) -> Self {
        Task::GroundRoute(t)
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

    /// The route goes by formation name and m/s; unset speeds stay unset (the engine's
    /// default applies).
    #[test]
    fn ground_route_to_proto() {
        let a = MapPoint::new(1.0, 2.0);
        let b = MapPoint::new(3.0, 4.0);
        let t: Task = Task::ground_route([
            RoutePoint::on_road(a).speed(sortie_types::kmh(36)),
            RoutePoint::new(b, GroundFormation::EchelonRight),
            RoutePoint::on_railroads(b),
            RoutePoint::off_road(a),
        ])
        .into();
        let v1::set_task::Task::GroundRoute(r) = t.to_proto() else {
            panic!()
        };
        assert_eq!(r.speed_mps, None);
        let got: Vec<_> = r
            .points
            .iter()
            .map(|p| (p.point.unwrap().north, p.formation(), p.speed_mps))
            .collect();
        assert_eq!(
            got,
            [
                (1.0, v1::GroundFormation::OnRoad, Some(10.0)),
                (3.0, v1::GroundFormation::EchelonRight, None),
                (3.0, v1::GroundFormation::OnRailroads, None),
                (1.0, v1::GroundFormation::OffRoad, None),
            ]
        );
        let t: Task = Task::ground_route([RoutePoint::off_road(a)])
            .speed(sortie_types::mps(4))
            .into();
        let v1::set_task::Task::GroundRoute(r) = t.to_proto() else {
            panic!()
        };
        assert_eq!(r.speed_mps, Some(4.0));
        assert_eq!(GroundFormation::default(), GroundFormation::OffRoad);
    }
}
