//! The shared vocabulary of the wide API (API revision 1): one selector type per kind of
//! thing, plus places, areas and volumes. Builders take `impl Into<Sel>`, so a handle the
//! logic already has (an [`Eid`], a [`Group`], a world [`Entity`], a scenario handle such as
//! [`AirbaseRef`] or [`ZoneRef`], a [`Vec3`]) goes in as it is.
//!
//! Rules the engine applies to every selector:
//!
//! * a selector always names something: there is no "unset = everything" (everyone is
//!   [`Audience::all`]);
//! * by Eid (preferred) it is that object: if it is gone the command fails with
//!   `Error::Gone`; an Eid of another kind of thing (a group Eid where an object is wanted)
//!   is `Error::Invalid`;
//! * by name it is whoever holds the name in DCS when the command runs (after a respawn, the
//!   new object);
//! * by manifest symbol (`ident`) it is the object the manifest names;
//! * numbers must be finite.
//!
//! Each type converts to its `sortie.v1` message (`From`), for code that builds commands
//! itself.

use sortie_proto::v1;
use sortie_types::{AirbaseRef, Coalition, Eid, GroupRef, MapPoint, Vec3, ZoneRef};
use sortie_world::{Airbase, Entity};

use crate::group::{AnyGroup, Group};

macro_rules! proto_newtype {
    ($(#[$doc:meta])* $name:ident => $proto:ident) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name(v1::$proto);

        impl $name {
            /// The wire form.
            #[must_use]
            pub fn to_proto(&self) -> v1::$proto {
                self.0.clone()
            }

            /// From the wire form (a selector the engine sent, e.g. in a call's result).
            #[must_use]
            pub fn from_proto(p: v1::$proto) -> Self {
                Self(p)
            }
        }

        impl From<$name> for v1::$proto {
            fn from(s: $name) -> Self {
                s.0
            }
        }
    };
}

// ----- objects, groups, units -------------------------------------------------------------------

proto_newtype! {
    /// Any world object: a unit, a static, a cargo, a weapon in flight, ejected crew, or a
    /// map (scenery) object.
    Obj => ObjSel
}

impl Obj {
    #[must_use]
    pub fn eid(eid: Eid) -> Self {
        Self(v1::ObjSel {
            sel: Some(v1::obj_sel::Sel::Eid(eid.to_u64())),
        })
    }

    /// A unit by its DCS name.
    #[must_use]
    pub fn unit(name: impl Into<String>) -> Self {
        Self(v1::ObjSel {
            sel: Some(v1::obj_sel::Sel::UnitName(name.into())),
        })
    }

    /// A static object (or cargo) by its DCS name.
    #[must_use]
    pub fn static_object(name: impl Into<String>) -> Self {
        Self(v1::ObjSel {
            sel: Some(v1::obj_sel::Sel::StaticName(name.into())),
        })
    }

    /// A map object by its map object id (decimal digits, as events and queries report it).
    #[must_use]
    pub fn scenery(id: impl Into<String>) -> Self {
        Self(v1::ObjSel {
            sel: Some(v1::obj_sel::Sel::SceneryName(id.into())),
        })
    }

    /// The weapon of an event (`Shot.weapon`, `Hit.weapon`, `Kill.weapon`, `ctx.weapon`) by its
    /// Eid; `None` when the event named no weapon object (Eid 0: `weapon_name` only, or a
    /// unit in a collision). The Eid selects the weapon while DCS still has it (engine API
    /// revision 1).
    #[must_use]
    pub fn weapon(w: &v1::WeaponSnap) -> Option<Self> {
        Eid::from_u64(w.eid).map(Self::eid)
    }

    /// An object of an event snap by its Eid (an ejected pilot in `Unknown.ctx` included);
    /// `None` for Eid 0 (an object the engine could not name).
    #[must_use]
    pub fn snap(s: &v1::EntitySnap) -> Option<Self> {
        Eid::from_u64(s.eid).map(Self::eid)
    }
}

impl From<Eid> for Obj {
    fn from(e: Eid) -> Self {
        Obj::eid(e)
    }
}

impl From<&Entity> for Obj {
    fn from(e: &Entity) -> Self {
        Obj::eid(e.eid)
    }
}

proto_newtype! {
    /// A group: by Eid (a [`Group`] handle), DCS name or manifest symbol.
    GroupSel => GroupSel
}

impl GroupSel {
    #[must_use]
    pub fn eid(eid: Eid) -> Self {
        Self(v1::GroupSel {
            sel: Some(v1::group_sel::Sel::Eid(eid.to_u64())),
        })
    }

    #[must_use]
    pub fn name(name: impl Into<String>) -> Self {
        Self(v1::GroupSel {
            sel: Some(v1::group_sel::Sel::Name(name.into())),
        })
    }

    /// A group of the manifest by its symbol, e.g. `"SAM_NORTH"`.
    #[must_use]
    pub fn ident(ident: impl Into<String>) -> Self {
        Self(v1::GroupSel {
            sel: Some(v1::group_sel::Sel::Ident(ident.into())),
        })
    }
}

impl<K> From<&Group<K>> for GroupSel {
    fn from(g: &Group<K>) -> Self {
        Self(g.selector())
    }
}

impl From<Eid> for GroupSel {
    fn from(e: Eid) -> Self {
        GroupSel::eid(e)
    }
}

impl From<&AnyGroup> for GroupSel {
    fn from(g: &AnyGroup) -> Self {
        Self(g.selector())
    }
}

impl<M, K> From<GroupRef<M, K>> for GroupSel {
    fn from(g: GroupRef<M, K>) -> Self {
        GroupSel::ident(g.ident())
    }
}

proto_newtype! {
    /// A unit (airplane, helicopter, ground unit, ship): by Eid or DCS name.
    UnitSel => UnitSel
}

impl UnitSel {
    #[must_use]
    pub fn eid(eid: Eid) -> Self {
        Self(v1::UnitSel {
            sel: Some(v1::unit_sel::Sel::Eid(eid.to_u64())),
        })
    }

    #[must_use]
    pub fn name(name: impl Into<String>) -> Self {
        Self(v1::UnitSel {
            sel: Some(v1::unit_sel::Sel::Name(name.into())),
        })
    }
}

impl From<Eid> for UnitSel {
    fn from(e: Eid) -> Self {
        UnitSel::eid(e)
    }
}

impl From<&Entity> for UnitSel {
    fn from(e: &Entity) -> Self {
        UnitSel::eid(e.eid)
    }
}

proto_newtype! {
    /// An AI controller: the one of a group (the normal case) or of one unit.
    ControllerSel => ControllerSel
}

impl ControllerSel {
    #[must_use]
    pub fn group(g: impl Into<GroupSel>) -> Self {
        Self(v1::ControllerSel {
            sel: Some(v1::controller_sel::Sel::Group(g.into().0)),
        })
    }

    #[must_use]
    pub fn unit(u: impl Into<UnitSel>) -> Self {
        Self(v1::ControllerSel {
            sel: Some(v1::controller_sel::Sel::Unit(u.into().0)),
        })
    }
}

impl<K> From<&Group<K>> for ControllerSel {
    fn from(g: &Group<K>) -> Self {
        ControllerSel::group(g)
    }
}

// ----- airbases, warehouses, zones, players -----------------------------------------------------

proto_newtype! {
    /// An airbase, FARP or ship deck: by Eid (the airbase's, or the Eid of the static or ship
    /// that carries it), DCS name or manifest symbol. A dead base is gone, by name as well.
    AirbaseSel => AirbaseSel
}

impl AirbaseSel {
    #[must_use]
    pub fn eid(eid: Eid) -> Self {
        Self(v1::AirbaseSel {
            sel: Some(v1::airbase_sel::Sel::Eid(eid.to_u64())),
        })
    }

    #[must_use]
    pub fn name(name: impl Into<String>) -> Self {
        Self(v1::AirbaseSel {
            sel: Some(v1::airbase_sel::Sel::Name(name.into())),
        })
    }

    #[must_use]
    pub fn ident(ident: impl Into<String>) -> Self {
        Self(v1::AirbaseSel {
            sel: Some(v1::airbase_sel::Sel::Ident(ident.into())),
        })
    }
}

impl<M> From<AirbaseRef<M>> for AirbaseSel {
    fn from(a: AirbaseRef<M>) -> Self {
        AirbaseSel::ident(a.ident())
    }
}

impl From<Eid> for AirbaseSel {
    fn from(e: Eid) -> Self {
        AirbaseSel::eid(e)
    }
}

impl From<&Airbase> for AirbaseSel {
    fn from(a: &Airbase) -> Self {
        AirbaseSel::eid(a.eid)
    }
}

proto_newtype! {
    /// A warehouse: an airbase's, the one inside a cargo static, or one by its DCS name.
    WarehouseSel => WarehouseSel
}

impl WarehouseSel {
    #[must_use]
    pub fn airbase(a: impl Into<AirbaseSel>) -> Self {
        Self(v1::WarehouseSel {
            sel: Some(v1::warehouse_sel::Sel::Airbase(a.into().0)),
        })
    }

    #[must_use]
    pub fn cargo(o: impl Into<Obj>) -> Self {
        Self(v1::WarehouseSel {
            sel: Some(v1::warehouse_sel::Sel::Cargo(o.into().0)),
        })
    }

    /// `Warehouse.getByName(name)`.
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self(v1::WarehouseSel {
            sel: Some(v1::warehouse_sel::Sel::DcsName(name.into())),
        })
    }
}

impl<M> From<AirbaseRef<M>> for WarehouseSel {
    fn from(a: AirbaseRef<M>) -> Self {
        WarehouseSel::airbase(a)
    }
}

proto_newtype! {
    /// A trigger zone: by manifest symbol or mission-editor name.
    ZoneSel => ZoneSel
}

impl ZoneSel {
    #[must_use]
    pub fn ident(ident: impl Into<String>) -> Self {
        Self(v1::ZoneSel {
            sel: Some(v1::zone_sel::Sel::Ident(ident.into())),
        })
    }

    #[must_use]
    pub fn name(name: impl Into<String>) -> Self {
        Self(v1::ZoneSel {
            sel: Some(v1::zone_sel::Sel::Name(name.into())),
        })
    }
}

impl<M> From<ZoneRef<M>> for ZoneSel {
    fn from(z: ZoneRef<M>) -> Self {
        ZoneSel::ident(z.ident())
    }
}

proto_newtype! {
    /// A connected player: `ucid` is stable across reconnects, `player_id` is valid for one
    /// connection, `name` must match exactly one connected player.
    PlayerSel => PlayerSel
}

impl PlayerSel {
    #[must_use]
    pub fn ucid(ucid: impl Into<String>) -> Self {
        Self(v1::PlayerSel {
            sel: Some(v1::player_sel::Sel::Ucid(ucid.into())),
        })
    }

    #[must_use]
    pub fn player_id(id: u32) -> Self {
        Self(v1::PlayerSel {
            sel: Some(v1::player_sel::Sel::PlayerId(id)),
        })
    }

    #[must_use]
    pub fn name(name: impl Into<String>) -> Self {
        Self(v1::PlayerSel {
            sel: Some(v1::player_sel::Sel::Name(name.into())),
        })
    }
}

proto_newtype! {
    /// Who sees or hears something. Each operation says which members it accepts; one it
    /// cannot serve is `Error::Invalid`, never a wider audience.
    Audience => Audience
}

impl Audience {
    /// Everyone.
    #[must_use]
    pub fn all() -> Self {
        Self(v1::Audience {
            to: Some(v1::audience::To::All(true)),
        })
    }

    #[must_use]
    pub fn coalition(c: Coalition) -> Self {
        Self(v1::Audience {
            to: Some(v1::audience::To::Coalition(c.as_i32())),
        })
    }

    /// A DCS country key as in `country.id`, e.g. `"USA"`.
    #[must_use]
    pub fn country(key: impl Into<String>) -> Self {
        Self(v1::Audience {
            to: Some(v1::audience::To::Country(key.into())),
        })
    }

    /// The players in a group (DCS reaches occupied player units only).
    #[must_use]
    pub fn group(g: impl Into<GroupSel>) -> Self {
        Self(v1::Audience {
            to: Some(v1::audience::To::Group(g.into().0)),
        })
    }

    #[must_use]
    pub fn unit(u: impl Into<UnitSel>) -> Self {
        Self(v1::Audience {
            to: Some(v1::audience::To::Unit(u.into().0)),
        })
    }

    #[must_use]
    pub fn player(p: PlayerSel) -> Self {
        Self(v1::Audience {
            to: Some(v1::audience::To::Player(p.0)),
        })
    }
}

impl From<Coalition> for Audience {
    fn from(c: Coalition) -> Self {
        Audience::coalition(c)
    }
}

impl<K> From<&Group<K>> for Audience {
    fn from(g: &Group<K>) -> Self {
        Audience::group(g)
    }
}

// ----- places, areas, volumes -------------------------------------------------------------------

proto_newtype! {
    /// A place: a fixed point, a point on the ground, an object (its position when the
    /// command runs) or the centre of a zone; [`Anchor::up`] adds height.
    Anchor => Anchor
}

impl Anchor {
    /// Used as given.
    #[must_use]
    pub fn point(p: Vec3) -> Self {
        Self(v1::Anchor {
            at: Some(v1::anchor::At::Point(p.into())),
            height_offset_m: 0.0,
        })
    }

    /// On the terrain, height looked up in DCS when the command runs.
    #[must_use]
    pub fn ground(p: MapPoint) -> Self {
        Self(v1::Anchor {
            at: Some(v1::anchor::At::Ground(p.into())),
            height_offset_m: 0.0,
        })
    }

    /// The object's position when the command runs (gone: `Error::Gone`).
    #[must_use]
    pub fn object(o: impl Into<Obj>) -> Self {
        Self(v1::Anchor {
            at: Some(v1::anchor::At::Object(o.into().0)),
            height_offset_m: 0.0,
        })
    }

    /// The zone's centre on the ground.
    #[must_use]
    pub fn zone(z: impl Into<ZoneSel>) -> Self {
        Self(v1::Anchor {
            at: Some(v1::anchor::At::Zone(z.into().0)),
            height_offset_m: 0.0,
        })
    }

    /// Metres added to the height of the resolved point (may be negative).
    #[must_use]
    pub fn up(mut self, metres: f64) -> Self {
        self.0.height_offset_m = metres;
        self
    }
}

impl From<Vec3> for Anchor {
    fn from(p: Vec3) -> Self {
        Anchor::point(p)
    }
}

impl From<MapPoint> for Anchor {
    fn from(p: MapPoint) -> Self {
        Anchor::ground(p)
    }
}

impl From<Obj> for Anchor {
    fn from(o: Obj) -> Self {
        Anchor::object(o)
    }
}

impl From<&Entity> for Anchor {
    fn from(e: &Entity) -> Self {
        Anchor::object(e)
    }
}

impl<M> From<ZoneRef<M>> for Anchor {
    fn from(z: ZoneRef<M>) -> Self {
        Anchor::zone(z)
    }
}

proto_newtype! {
    /// A 2D area on the map.
    MapArea => MapArea
}

impl MapArea {
    #[must_use]
    pub fn zone(z: impl Into<ZoneSel>) -> Self {
        Self(v1::MapArea {
            area: Some(v1::map_area::Area::Zone(z.into().0)),
        })
    }

    #[must_use]
    pub fn circle(center: MapPoint, radius_m: f64) -> Self {
        Self(v1::MapArea {
            area: Some(v1::map_area::Area::Circle(v1::MapCircle {
                center: Some(center.into()),
                radius_m,
            })),
        })
    }

    /// 3 to 64 vertices in order; closed implicitly.
    #[must_use]
    pub fn polygon(points: impl IntoIterator<Item = MapPoint>) -> Self {
        Self(v1::MapArea {
            area: Some(v1::map_area::Area::Polygon(v1::MapPolygon {
                points: points.into_iter().map(Into::into).collect(),
            })),
        })
    }

    /// A circle around the object's position, evaluated when it is used.
    #[must_use]
    pub fn around(o: impl Into<Obj>, radius_m: f64) -> Self {
        Self(v1::MapArea {
            area: Some(v1::map_area::Area::Around(v1::AroundObject {
                object: Some(o.into().0),
                radius_m,
            })),
        })
    }
}

impl<M> From<ZoneRef<M>> for MapArea {
    fn from(z: ZoneRef<M>) -> Self {
        MapArea::zone(z)
    }
}

proto_newtype! {
    /// A 3D volume: the DCS volume types (sphere, box, segment, pyramid) and a map area
    /// between two heights.
    Volume => Volume
}

impl Volume {
    #[must_use]
    pub fn sphere(center: Vec3, radius_m: f64) -> Self {
        Self::shape(v1::volume::Shape::Sphere(v1::Sphere {
            center: Some(center.into()),
            radius_m,
            center_on_ground: false,
        }))
    }

    /// A sphere whose centre is on the terrain at `center`.
    #[must_use]
    pub fn sphere_on_ground(center: MapPoint, radius_m: f64) -> Self {
        Self::shape(v1::volume::Shape::Sphere(v1::Sphere {
            center: Some(Vec3::new(center.north, 0.0, center.east).into()),
            radius_m,
            center_on_ground: true,
        }))
    }

    /// Axis-aligned box between two corners.
    #[must_use]
    pub fn axis_box(min: Vec3, max: Vec3) -> Self {
        Self::shape(v1::volume::Shape::AxisBox(v1::AxisBox {
            min: Some(min.into()),
            max: Some(max.into()),
        }))
    }

    #[must_use]
    pub fn segment(start: Vec3, end: Vec3) -> Self {
        Self::shape(v1::volume::Shape::Segment(v1::Segment {
            start: Some(start.into()),
            end: Some(end.into()),
        }))
    }

    /// A pyramid from `apex` along heading and pitch (radians), `length_m` long, with the
    /// given horizontal and vertical half angles (radians, below pi/2).
    #[must_use]
    pub fn pyramid(
        apex: Vec3,
        heading_rad: f64,
        pitch_rad: f64,
        length_m: f64,
        half_angles_rad: (f64, f64),
    ) -> Self {
        Self::shape(v1::volume::Shape::Pyramid(v1::Pyramid {
            apex: Some(apex.into()),
            heading_rad,
            pitch_rad,
            length_m,
            half_angle_hor_rad: half_angles_rad.0,
            half_angle_ver_rad: half_angles_rad.1,
        }))
    }

    /// A map area from 200 m below its lowest to 20 000 m above its highest terrain point;
    /// [`Volume::between`] sets the heights.
    #[must_use]
    pub fn area(area: impl Into<MapArea>) -> Self {
        Self::shape(v1::volume::Shape::Area(v1::AreaVolume {
            area: Some(area.into().0),
            floor_m: None,
            ceiling_m: None,
        }))
    }

    /// Heights above sea level of an [`Volume::area`] volume; no effect on the other shapes.
    #[must_use]
    pub fn between(mut self, floor_m: f64, ceiling_m: f64) -> Self {
        if let Some(v1::volume::Shape::Area(a)) = &mut self.0.shape {
            a.floor_m = Some(floor_m);
            a.ceiling_m = Some(ceiling_m);
        }
        self
    }

    fn shape(s: v1::volume::Shape) -> Self {
        Self(v1::Volume { shape: Some(s) })
    }
}

#[cfg(test)]
mod tests {
    use sortie_types::Ground;

    use super::*;

    #[derive(Clone, Copy, Debug, Default)]
    struct Op;

    const SAM: GroupRef<Op, Ground> = GroupRef::new("SAM_NORTH");
    const MAYKOP: AirbaseRef<Op> = AirbaseRef::new("MAYKOP");
    const RING: ZoneRef<Op> = ZoneRef::circle("SAM_RING", MapPoint::new(0.0, 0.0), 1000.0);

    fn eid(n: u32) -> Eid {
        Eid::from_u64(sortie_proto::eid::pack(n, 1)).unwrap()
    }

    #[test]
    fn handles_convert_to_the_selector_of_their_kind() {
        let o: Obj = eid(3).into();
        assert_eq!(
            v1::ObjSel::from(o),
            v1::ObjSel {
                sel: Some(v1::obj_sel::Sel::Eid(eid(3).to_u64()))
            }
        );
        let g: GroupSel = SAM.into();
        assert_eq!(
            g.to_proto().sel,
            Some(v1::group_sel::Sel::Ident("SAM_NORTH".into()))
        );
        let a: AirbaseSel = MAYKOP.into();
        assert_eq!(
            a.to_proto().sel,
            Some(v1::airbase_sel::Sel::Ident("MAYKOP".into()))
        );
        let w: WarehouseSel = MAYKOP.into();
        assert!(matches!(
            w.to_proto().sel,
            Some(v1::warehouse_sel::Sel::Airbase(_))
        ));
        let z: ZoneSel = RING.into();
        assert_eq!(
            z.to_proto().sel,
            Some(v1::zone_sel::Sel::Ident("SAM_RING".into()))
        );
        let mut e = Entity::new(eid(9), 1);
        e.name = "SAM North-1".into();
        assert_eq!(
            UnitSel::from(&e).to_proto().sel,
            Some(v1::unit_sel::Sel::Eid(eid(9).to_u64()))
        );
        assert_eq!(
            Obj::from(&e).to_proto().sel,
            Some(v1::obj_sel::Sel::Eid(eid(9).to_u64()))
        );
        let any = AnyGroup {
            eid: None,
            name: "Blue 1".into(),
        };
        assert_eq!(
            GroupSel::from(&any).to_proto().sel,
            Some(v1::group_sel::Sel::Name("Blue 1".into()))
        );
    }

    #[test]
    fn places_and_audiences() {
        let a: Anchor = Vec3::new(1.0, 2.0, 3.0).into();
        assert!(matches!(a.to_proto().at, Some(v1::anchor::At::Point(_))));
        let a: Anchor = MapPoint::new(1.0, 2.0).into();
        assert!(matches!(a.to_proto().at, Some(v1::anchor::At::Ground(_))));
        let a = Anchor::zone(RING).up(30.0).to_proto();
        assert!(matches!(a.at, Some(v1::anchor::At::Zone(_))));
        assert_eq!(a.height_offset_m, 30.0);
        let a: Anchor = Obj::unit("SAM North-1").into();
        assert!(matches!(a.to_proto().at, Some(v1::anchor::At::Object(_))));
        assert_eq!(
            Audience::from(Coalition::Blue).to_proto().to,
            Some(v1::audience::To::Coalition(Coalition::Blue.as_i32()))
        );
        assert_eq!(
            Audience::all().to_proto().to,
            Some(v1::audience::To::All(true))
        );
        assert!(matches!(
            ControllerSel::group(SAM).to_proto().sel,
            Some(v1::controller_sel::Sel::Group(_))
        ));
        assert!(matches!(
            Audience::player(PlayerSel::ucid("abc")).to_proto().to,
            Some(v1::audience::To::Player(_))
        ));
    }

    #[test]
    fn areas_and_volumes() {
        let poly = MapArea::polygon([
            MapPoint::new(0.0, 0.0),
            MapPoint::new(0.0, 1.0),
            MapPoint::new(1.0, 1.0),
        ]);
        let Some(v1::map_area::Area::Polygon(p)) = poly.to_proto().area else {
            panic!("a polygon")
        };
        assert_eq!(p.points.len(), 3);
        let open = Volume::area(RING).to_proto();
        let Some(v1::volume::Shape::Area(a)) = &open.shape else {
            panic!("an area volume")
        };
        assert_eq!((a.floor_m, a.ceiling_m), (None, None));
        let banded =
            Volume::area(MapArea::circle(MapPoint::new(0.0, 0.0), 50.0)).between(0.0, 500.0);
        let Some(v1::volume::Shape::Area(a)) = banded.to_proto().shape else {
            panic!("an area volume")
        };
        assert_eq!((a.floor_m, a.ceiling_m), (Some(0.0), Some(500.0)));
        let s = Volume::sphere_on_ground(MapPoint::new(5.0, 6.0), 10.0).to_proto();
        let Some(v1::volume::Shape::Sphere(s)) = s.shape else {
            panic!("a sphere")
        };
        assert!(s.center_on_ground);
        assert_eq!(s.center.map(|c| (c.x, c.z)), Some((5.0, 6.0)));
        // `between` leaves the DCS shapes alone.
        let b = Volume::axis_box(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(b.clone().between(1.0, 2.0), b);
    }
}
