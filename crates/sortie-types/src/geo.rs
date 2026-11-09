//! Geometry in DCS axes.
//!
//! * [`MapPoint`] — 2D map position with explicit names: `north` (editor/runtime x) and
//!   `east` (editor y, runtime z), metres.
//! * [`Vec3`] — runtime position: `x` north, `y` up (metres above sea level), `z` east.
//! * [`ZoneShape`] — circle or polygon on the map.

use std::f64::consts::TAU;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};
use sortie_proto::v1;

use crate::units::Length;

/// 2D map point, metres. `north` = DCS x, `east` = DCS editor y / runtime z.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MapPoint {
    pub north: f64,
    pub east: f64,
}

impl MapPoint {
    #[must_use]
    pub const fn new(north: f64, east: f64) -> Self {
        Self { north, east }
    }

    /// The 3D point at altitude `alt` (above sea level) over this map point.
    #[must_use]
    pub const fn alt(self, alt: Length) -> Vec3 {
        Vec3 {
            x: self.north,
            y: alt.meters(),
            z: self.east,
        }
    }

    /// Straight-line map distance in metres.
    #[must_use]
    pub fn distance(self, other: MapPoint) -> f64 {
        (self.north - other.north).hypot(self.east - other.east)
    }

    /// This point moved by the given offsets (metres).
    #[must_use]
    pub const fn offset(self, north_m: f64, east_m: f64) -> Self {
        Self {
            north: self.north + north_m,
            east: self.east + east_m,
        }
    }

    /// True bearing from `self` to `other` in radians: 0 = north, π/2 = east, in `[0, 2π)`.
    /// Returns 0 for identical points.
    #[must_use]
    pub fn bearing_to(self, other: MapPoint) -> f64 {
        let b = (other.east - self.east).atan2(other.north - self.north);
        if b < 0.0 { b + TAU } else { b }
    }

    /// The point `distance` away along the true bearing `bearing_rad` (0 = north, clockwise).
    #[must_use]
    pub fn toward(self, bearing_rad: f64, distance: Length) -> Self {
        let d = distance.meters();
        Self {
            north: self.north + d * bearing_rad.cos(),
            east: self.east + d * bearing_rad.sin(),
        }
    }

    /// Linear interpolation: `t = 0` is `self`, `t = 1` is `other`.
    #[must_use]
    pub fn lerp(self, other: MapPoint, t: f64) -> Self {
        Self {
            north: self.north + (other.north - self.north) * t,
            east: self.east + (other.east - self.east) * t,
        }
    }

    /// Both coordinates are finite (not NaN or infinite).
    #[must_use]
    pub const fn is_finite(self) -> bool {
        self.north.is_finite() && self.east.is_finite()
    }
}

/// Runtime 3D point: x = north, y = up, z = east (metres).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    #[must_use]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// The map point under this position.
    #[must_use]
    pub const fn map_point(self) -> MapPoint {
        MapPoint {
            north: self.x,
            east: self.z,
        }
    }

    /// Altitude above sea level.
    #[must_use]
    pub const fn alt(self) -> Length {
        Length::from_meters(self.y)
    }

    #[must_use]
    pub fn distance(self, other: Vec3) -> f64 {
        (self - other).length()
    }

    /// Distance on the map plane (ignores altitude).
    #[must_use]
    pub fn horizontal_distance(self, other: Vec3) -> f64 {
        self.map_point().distance(other.map_point())
    }

    /// Euclidean norm (for a velocity: the speed in m/s).
    #[must_use]
    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Dot product.
    #[must_use]
    pub fn dot(self, other: Vec3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// True heading of a horizontal direction vector (e.g. a velocity), radians in `[0, 2π)`.
    #[must_use]
    pub fn heading(self) -> f64 {
        MapPoint::default().bearing_to(self.map_point())
    }

    /// Every component is finite.
    #[must_use]
    pub const fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, o: Vec3) {
        *self = *self - o;
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, k: f64) -> Vec3 {
        Vec3::new(self.x * k, self.y * k, self.z * k)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

/// An axis-aligned rectangle on the map (inclusive), used for spatial indexing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: MapPoint,
    pub max: MapPoint,
}

impl Bounds {
    /// The smallest rectangle containing every point; `None` for no points or non-finite ones.
    #[must_use]
    pub fn of_points(points: &[MapPoint]) -> Option<Self> {
        let (first, rest) = points.split_first()?;
        let mut b = Bounds {
            min: *first,
            max: *first,
        };
        for p in rest {
            b.min.north = b.min.north.min(p.north);
            b.min.east = b.min.east.min(p.east);
            b.max.north = b.max.north.max(p.north);
            b.max.east = b.max.east.max(p.east);
        }
        (b.min.is_finite() && b.max.is_finite()).then_some(b)
    }

    #[must_use]
    pub fn contains(&self, p: MapPoint) -> bool {
        p.north >= self.min.north
            && p.north <= self.max.north
            && p.east >= self.min.east
            && p.east <= self.max.east
    }

    #[must_use]
    pub fn intersects(&self, other: &Bounds) -> bool {
        self.min.north <= other.max.north
            && other.min.north <= self.max.north
            && self.min.east <= other.max.east
            && other.min.east <= self.max.east
    }

    /// The four corners.
    #[must_use]
    pub const fn corners(&self) -> [MapPoint; 4] {
        [
            self.min,
            MapPoint::new(self.min.north, self.max.east),
            self.max,
            MapPoint::new(self.max.north, self.min.east),
        ]
    }
}

impl From<MapPoint> for v1::MapPoint {
    fn from(p: MapPoint) -> Self {
        Self {
            north: p.north,
            east: p.east,
        }
    }
}

impl From<v1::MapPoint> for MapPoint {
    fn from(p: v1::MapPoint) -> Self {
        Self {
            north: p.north,
            east: p.east,
        }
    }
}

impl From<Vec3> for v1::Vec3 {
    fn from(p: Vec3) -> Self {
        Self {
            x: p.x,
            y: p.y,
            z: p.z,
        }
    }
}

impl From<v1::Vec3> for Vec3 {
    fn from(p: v1::Vec3) -> Self {
        Self {
            x: p.x,
            y: p.y,
            z: p.z,
        }
    }
}

/// A zone shape on the map. `'a` is `'static` in generated handles and borrows from the world
/// for zones loaded from a manifest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ZoneShape<'a> {
    Circle {
        center: MapPoint,
        radius_m: f64,
    },
    /// Vertices in order; closed implicitly.
    Polygon {
        points: &'a [MapPoint],
    },
}

impl ZoneShape<'_> {
    /// `true` if `p` lies inside or on the edge of the shape (map plane: runtime x/z).
    /// An invalid shape (see [`ZoneShape::is_valid`]) contains nothing.
    #[must_use]
    pub fn contains(&self, p: MapPoint) -> bool {
        match *self {
            ZoneShape::Circle { center, radius_m } => center.distance(p) <= radius_m,
            ZoneShape::Polygon { points } => polygon_contains(points, p),
        }
    }

    /// A circle needs a finite centre and a finite radius > 0; a polygon at least 3 finite
    /// vertices and a non-zero area.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        match *self {
            ZoneShape::Circle { center, radius_m } => {
                center.is_finite() && radius_m.is_finite() && radius_m > 0.0
            }
            ZoneShape::Polygon { points } => {
                points.len() >= 3
                    && points.iter().all(|p| p.is_finite())
                    && polygon_area(points).abs() > 0.0
            }
        }
    }

    /// Bounding rectangle; `None` for an invalid shape.
    #[must_use]
    pub fn bounds(&self) -> Option<Bounds> {
        if !self.is_valid() {
            return None;
        }
        match *self {
            ZoneShape::Circle { center, radius_m } => Some(Bounds {
                min: center.offset(-radius_m, -radius_m),
                max: center.offset(radius_m, radius_m),
            }),
            ZoneShape::Polygon { points } => Bounds::of_points(points),
        }
    }

    /// `true` if the whole rectangle lies inside the shape (exact for circles, conservative
    /// for polygons: `false` when unsure).
    #[must_use]
    pub fn contains_bounds(&self, b: &Bounds) -> bool {
        match *self {
            ZoneShape::Circle { .. } => b.corners().iter().all(|c| self.contains(*c)),
            // A polygon may be concave: corners inside do not prove the rectangle is inside.
            ZoneShape::Polygon { .. } => false,
        }
    }

    /// Circle centre, or the vertex average of a polygon.
    #[must_use]
    pub fn center(&self) -> MapPoint {
        match *self {
            ZoneShape::Circle { center, .. } => center,
            ZoneShape::Polygon { points: [] } => MapPoint::default(),
            ZoneShape::Polygon { points } => {
                let n = points.len() as f64;
                let (sn, se) = points
                    .iter()
                    .fold((0.0, 0.0), |(a, b), p| (a + p.north, b + p.east));
                MapPoint::new(sn / n, se / n)
            }
        }
    }
}

/// Shoelace formula (signed; north/east axes).
fn polygon_area(points: &[MapPoint]) -> f64 {
    let n = points.len();
    let twice: f64 = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a.north * b.east - b.north * a.east
        })
        .sum();
    twice / 2.0
}

fn on_segment(a: MapPoint, b: MapPoint, p: MapPoint) -> bool {
    let cross = (b.north - a.north) * (p.east - a.east) - (b.east - a.east) * (p.north - a.north);
    cross == 0.0
        && p.north >= a.north.min(b.north)
        && p.north <= a.north.max(b.north)
        && p.east >= a.east.min(b.east)
        && p.east <= a.east.max(b.east)
}

fn polygon_contains(points: &[MapPoint], p: MapPoint) -> bool {
    // Even-odd ray casting along +east; points exactly on an edge count as inside.
    let mut inside = false;
    let n = points.len();
    if n < 3 || !p.is_finite() {
        return false;
    }
    if (0..n).any(|i| on_segment(points[i], points[(i + 1) % n], p)) {
        return true;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (points[i], points[j]);
        if (a.north > p.north) != (b.north > p.north) {
            let t = (p.north - a.north) / (b.north - a.north);
            if p.east < a.east + t * (b.east - a.east) {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Anything that names a zone and can give its shape. World queries look the ident up in the
/// running manifest first and fall back to [`AsZone::zone_shape`].
pub trait AsZone {
    fn zone_ident(&self) -> &str;
    fn zone_shape(&self) -> ZoneShape<'_>;

    /// The shape, if it lives for `'static` (generated `ZoneRef` constants): lets queries keep
    /// it without copying polygon vertices.
    fn zone_shape_static(&self) -> Option<ZoneShape<'static>> {
        None
    }
}

impl<T: AsZone + ?Sized> AsZone for &T {
    fn zone_ident(&self) -> &str {
        (**self).zone_ident()
    }

    fn zone_shape(&self) -> ZoneShape<'_> {
        (**self).zone_shape()
    }

    fn zone_shape_static(&self) -> Option<ZoneShape<'static>> {
        (**self).zone_shape_static()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::ft;

    #[test]
    fn axes() {
        let p = MapPoint::new(-32798.0, 463776.0);
        let v = p.alt(ft(20_000));
        assert_eq!(v.x, -32798.0);
        assert_eq!(v.z, 463776.0);
        assert!((v.y - 6096.0).abs() < 1e-9);
        assert_eq!(v.map_point(), p);
    }

    #[test]
    fn shapes() {
        let c = ZoneShape::Circle {
            center: MapPoint::new(0.0, 0.0),
            radius_m: 100.0,
        };
        assert!(c.contains(MapPoint::new(60.0, 80.0)));
        assert!(!c.contains(MapPoint::new(60.0, 81.0)));
        static SQUARE: [MapPoint; 4] = [
            MapPoint::new(0.0, 0.0),
            MapPoint::new(0.0, 10.0),
            MapPoint::new(10.0, 10.0),
            MapPoint::new(10.0, 0.0),
        ];
        let s = ZoneShape::Polygon { points: &SQUARE };
        assert!(s.contains(MapPoint::new(5.0, 5.0)));
        assert!(!s.contains(MapPoint::new(15.0, 5.0)));
        assert_eq!(s.center(), MapPoint::new(5.0, 5.0));
        assert!(s.contains(MapPoint::new(0.0, 5.0)), "edges are inside");
        assert!(s.contains(MapPoint::new(10.0, 10.0)), "vertices are inside");
        assert!(s.is_valid());
        let b = s.bounds().unwrap();
        assert_eq!(b.min, MapPoint::new(0.0, 0.0));
        assert_eq!(b.max, MapPoint::new(10.0, 10.0));
        assert!(!s.contains(MapPoint::new(f64::NAN, 5.0)));
    }

    #[test]
    fn concave_polygon() {
        // An "L": the notch at (7.5, 7.5) is outside.
        static L: [MapPoint; 6] = [
            MapPoint::new(0.0, 0.0),
            MapPoint::new(10.0, 0.0),
            MapPoint::new(10.0, 5.0),
            MapPoint::new(5.0, 5.0),
            MapPoint::new(5.0, 10.0),
            MapPoint::new(0.0, 10.0),
        ];
        let s = ZoneShape::Polygon { points: &L };
        assert!(s.contains(MapPoint::new(2.0, 8.0)));
        assert!(s.contains(MapPoint::new(8.0, 2.0)));
        assert!(!s.contains(MapPoint::new(7.5, 7.5)));
        let cell = Bounds {
            min: MapPoint::new(1.0, 1.0),
            max: MapPoint::new(2.0, 2.0),
        };
        assert!(!s.contains_bounds(&cell), "conservative for polygons");
    }

    #[test]
    fn invalid_shapes_contain_nothing() {
        let zero = ZoneShape::Circle {
            center: MapPoint::default(),
            radius_m: 0.0,
        };
        assert!(!zero.is_valid());
        assert_eq!(zero.bounds(), None);
        let nan = ZoneShape::Circle {
            center: MapPoint::default(),
            radius_m: f64::NAN,
        };
        assert!(!nan.contains(MapPoint::default()));
        static LINE: [MapPoint; 3] = [
            MapPoint::new(0.0, 0.0),
            MapPoint::new(1.0, 1.0),
            MapPoint::new(2.0, 2.0),
        ];
        assert!(!ZoneShape::Polygon { points: &LINE }.is_valid());
        let c = ZoneShape::Circle {
            center: MapPoint::default(),
            radius_m: 100.0,
        };
        let inner = Bounds {
            min: MapPoint::new(-10.0, -10.0),
            max: MapPoint::new(10.0, 10.0),
        };
        let outer = Bounds {
            min: MapPoint::new(-90.0, -90.0),
            max: MapPoint::new(90.0, 90.0),
        };
        assert!(c.contains_bounds(&inner));
        assert!(!c.contains_bounds(&outer));
    }

    #[test]
    fn bearings_and_vectors() {
        use std::f64::consts::{FRAC_PI_2, PI};
        let o = MapPoint::new(0.0, 0.0);
        assert_eq!(o.bearing_to(MapPoint::new(10.0, 0.0)), 0.0);
        assert!((o.bearing_to(MapPoint::new(0.0, 10.0)) - FRAC_PI_2).abs() < 1e-12);
        assert!((o.bearing_to(MapPoint::new(0.0, -10.0)) - 3.0 * FRAC_PI_2).abs() < 1e-12);
        let p = o.toward(PI, crate::units::m(100));
        assert!((p.north + 100.0).abs() < 1e-9 && p.east.abs() < 1e-9);
        assert_eq!(
            o.lerp(MapPoint::new(10.0, 20.0), 0.5),
            MapPoint::new(5.0, 10.0)
        );
        let v = Vec3::new(3.0, 0.0, 4.0);
        assert_eq!(v.length(), 5.0);
        assert_eq!(v + v - v, v);
        assert_eq!(v * 2.0, Vec3::new(6.0, 0.0, 8.0));
        assert_eq!(-v, Vec3::new(-3.0, 0.0, -4.0));
        assert_eq!(v.dot(Vec3::new(1.0, 1.0, 1.0)), 7.0);
        assert_eq!(
            Vec3::new(0.0, 500.0, 0.0).horizontal_distance(Vec3::default()),
            0.0
        );
        assert!((Vec3::new(0.0, 0.0, 1.0).heading() - FRAC_PI_2).abs() < 1e-12);
    }
}
