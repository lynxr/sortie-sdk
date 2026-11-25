use super::*;
#[cfg(feature = "link")]
use sortie_proto::link::v1 as link;
use sortie_proto::v1;
use sortie_types::{Ground, GroupRef, ZoneRef};

#[derive(Clone, Copy, Debug, Default)]
struct Op;

const RING: ZoneRef<Op> = ZoneRef::circle("SAM_RING", MapPoint::new(0.0, 0.0), 1000.0);

fn unit(
    rid: u32,
    name: &str,
    ty: &str,
    group: &str,
    coalition: Coalition,
    category: Category,
) -> Observation {
    Observation {
        runtime_id: Some(rid),
        name: Some(name.into()),
        type_name: Some(ty.into()),
        coalition: Some(coalition),
        category: Some(category),
        group_name: Some(group.into()),
        pos: Some(Vec3::new(5000.0, 100.0, 5000.0)),
        ..Observation::default()
    }
}

#[test]
fn duplicate_birth_is_the_same_entity() {
    let mut w = World::new("s");
    let g = w.observe_group(
        "Blue 1",
        None,
        Some(Coalition::Blue),
        Some(Category::Airplane),
        Some(2),
    );
    let a = w.observe(&unit(
        10,
        "Blue 1-1",
        "F-16C_50",
        "Blue 1",
        Coalition::Blue,
        Category::Airplane,
    ));
    let b = w.observe(&unit(
        10,
        "Blue 1-1",
        "F-16C_50",
        "Blue 1",
        Coalition::Blue,
        Category::Airplane,
    ));
    assert!(matches!(a, Observed::New(_)));
    assert_eq!(b, Observed::Same(a.eid()));
    assert_eq!(w.group(&g).alive_count(), 1);
    assert_eq!(w.entity(a.eid()).unwrap().group, Some(g));
}

#[test]
fn reused_runtime_id_gets_a_fresh_eid() {
    let mut w = World::new("s");
    let a = w
        .observe(&unit(
            10,
            "Blue 1-1",
            "F-16C_50",
            "Blue 1",
            Coalition::Blue,
            Category::Airplane,
        ))
        .eid();
    let b = w.observe(&unit(
        10,
        "Red 1-1",
        "Su-27",
        "Red 1",
        Coalition::Red,
        Category::Airplane,
    ));
    assert_eq!(
        b,
        Observed::Reused {
            old: a,
            new: b.eid()
        }
    );
    assert_ne!(a, b.eid());
    assert!(w.entity(a).unwrap().dead);
    assert_eq!(w.entity_by_runtime_id(10).unwrap().eid, b.eid());
}

#[test]
fn respawned_group_gets_a_fresh_eid() {
    let mut w = World::new("s");
    let g1 = w.observe_group(
        "Red CAP 1",
        None,
        Some(Coalition::Red),
        Some(Category::Airplane),
        None,
    );
    let u = w
        .observe(&unit(
            20,
            "Red CAP 1-1",
            "Su-27",
            "Red CAP 1",
            Coalition::Red,
            Category::Airplane,
        ))
        .eid();
    assert!(w.mark_dead(u));
    assert!(w.group(&g1).is_dead());
    let g2 = w.observe_group(
        "Red CAP 1",
        None,
        Some(Coalition::Red),
        Some(Category::Airplane),
        None,
    );
    assert_ne!(g1, g2);
    assert_eq!(w.group("Red CAP 1").eid(), Some(g2));
}

#[test]
fn queries_filter_by_coalition_category_and_zone() {
    let mut w = World::new("s");
    w.observe(&unit(
        1,
        "Blue 1-1",
        "F-16C_50",
        "Blue 1",
        Coalition::Blue,
        Category::Airplane,
    ));
    let mut sam = unit(
        2,
        "SAM-1",
        "S-300PS 40B6M tr",
        "SAM",
        Coalition::Red,
        Category::GroundUnit,
    );
    sam.pos = Some(Vec3::new(100.0, 0.0, 100.0));
    w.observe(&sam);
    assert_eq!(w.units().count(), 2);
    assert_eq!(w.units().coalition(Coalition::Blue).air().count(), 1);
    // The handle's own geometry (no manifest zones loaded): blue is 7 km away.
    assert!(!w.units().coalition(Coalition::Blue).air().any_inside(RING));
    assert!(w.units().ground().any_inside(RING));
    // Manifest geometry wins over the handle's.
    w.set_zones(&[ring_zone(60_000.0)]);
    assert!(w.units().coalition(Coalition::Blue).air().any_inside(RING));
    assert_eq!(w.units().in_zone(RING).count(), 2);
    // A shrunken manifest zone excludes the blue jet again.
    w.set_zones(&[ring_zone(1_000.0)]);
    assert_eq!(w.units().count_inside(RING), 1);
}

#[test]
fn snapshot_then_deltas_keep_a_replica_in_sync() {
    let mut engine = World::new("01JC6Z00000000000000007XQ2");
    let g = engine.observe_group(
        "SAM North",
        Some("SAM_NORTH"),
        Some(Coalition::Red),
        Some(Category::GroundUnit),
        Some(1),
    );
    let u1 = engine
        .observe(&unit(
            1,
            "SAM North-1",
            "S-300PS 54K6 cp",
            "SAM North",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid();
    let mut replica = World::default();
    replica.apply_snapshot(&engine.snapshot());
    assert_eq!(
        replica
            .group(&sortie_types::GroupRef::<Op, sortie_types::Ground>::new(
                "SAM_NORTH"
            ))
            .alive_count(),
        1
    );

    let from = engine.rev();
    let u2 = engine
        .observe(&unit(
            2,
            "SAM North-2",
            "S-300PS 40B6M tr",
            "SAM North",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid();
    engine.mark_dead(u1);
    replica.apply_delta(&engine.delta_since(from)).unwrap();
    assert_eq!(replica.rev(), engine.rev());
    assert_eq!(replica.group(&g).alive_count(), 1);
    assert_eq!(replica.group(&g).alive_units().next().unwrap().eid, u2);
    assert_eq!(replica.snapshot(), engine.snapshot());

    // A delta from the wrong revision is refused.
    let stale = engine.delta_since(0);
    assert!(matches!(
        replica.apply_delta(&stale),
        Err(WorldError::RevisionMismatch { .. })
    ));
}

#[test]
fn dead_event_marks_the_unit_dead_and_sweep_finds_missing() {
    let mut w = World::new("s");
    let a = w
        .observe(&unit(
            1,
            "A",
            "T-72B",
            "G",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid();
    let b = w
        .observe(&unit(
            2,
            "B",
            "T-72B",
            "G",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid();
    let snap = w.entity(a).unwrap().to_snap();
    w.apply_event(&v1::Event {
        model_time: 12.0,
        kind: Some(v1::event::Kind::Dead(v1::Dead {
            initiator: Some(snap),
        })),
        ..Default::default()
    });
    assert!(w.entity(a).unwrap().dead);
    assert_eq!(w.model_time(), 12.0);

    // A full sweep that does not see B (its DEAD was lost) retires it.
    w.begin_sweep(7);
    #[cfg(feature = "link")]
    w.apply_sample(&sortie_proto::link::v1::Sample {
        runtime_id: 99,
        alive: true,
        ..Default::default()
    });
    assert_eq!(w.end_sweep(7, |_| true), vec![b]);
    assert!(w.entity(b).unwrap().dead);
    assert_eq!(w.units().count(), 0);
    assert_eq!(w.units().including_dead().count(), 2);
}

#[test]
fn eids_come_from_the_slotmap_layout() {
    let mut w = World::new("s");
    let e = w
        .observe(&Observation {
            name: Some("x".into()),
            ..Observation::default()
        })
        .eid();
    assert_eq!(e.generation(), 1);
    assert_eq!(
        sortie_proto::eid::pack(e.index(), e.generation()),
        e.to_u64()
    );
}

fn ring_zone(radius_m: f64) -> mpb::Zone {
    mpb::Zone {
        ident: "SAM_RING".into(),
        name: "SAM ring".into(),
        shape: Some(mpb::zone::Shape::Circle(mpb::Circle {
            center: Some(v1::MapPoint {
                north: 0.0,
                east: 0.0,
            }),
            radius_m,
        })),
    }
}

fn at(mut o: Observation, north: f64, east: f64) -> Observation {
    o.pos = Some(Vec3::new(north, 0.0, east));
    o
}

fn dead_event(snap: v1::EntitySnap) -> v1::Event {
    v1::Event {
        kind: Some(v1::event::Kind::Dead(v1::Dead {
            initiator: Some(snap),
        })),
        ..Default::default()
    }
}

#[test]
fn replica_events_do_not_move_the_revision() {
    let mut engine = World::new("s");
    engine.observe_group("G", None, Some(Coalition::Red), None, None);
    let a = engine
        .observe(&unit(
            1,
            "A",
            "T-72B",
            "G",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid();
    let mut replica = World::from_snapshot(&engine.snapshot());
    assert!(replica.is_replica());
    let base = engine.rev();

    // The engine decodes DEAD: mirror changes, the event goes out at once...
    engine.mark_dead(a);
    let ev = dead_event(engine.entity(a).unwrap().to_snap());
    replica.apply_event(&ev);
    assert!(replica.entity(a).unwrap().dead, "user code sees the death");
    assert_eq!(replica.rev(), base, "the replica revision is the engine's");

    // ... and the next tick's delta still applies.
    replica.apply_delta(&engine.delta_since(base)).unwrap();
    assert_eq!(replica.snapshot(), engine.snapshot());
}

#[test]
fn death_is_final_even_against_stale_snaps() {
    let mut engine = World::new("s");
    let a = engine
        .observe(&unit(
            1,
            "A",
            "T-72B",
            "G",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid();
    let alive_snap = engine.entity(a).unwrap().to_snap();
    let mut replica = World::from_snapshot(&engine.snapshot());
    let base = engine.rev();
    engine.mark_dead(a);
    replica.apply_delta(&engine.delta_since(base)).unwrap();
    // A late event carrying the pre-death snap does not resurrect the unit.
    replica.apply_event(&v1::Event {
        kind: Some(v1::event::Kind::Hit(v1::Hit {
            target: Some(alive_snap),
            ..Default::default()
        })),
        ..Default::default()
    });
    assert!(replica.entity(a).unwrap().dead);
    assert_eq!(replica.units().count(), 0);
}

#[test]
fn birth_with_the_id_of_a_dead_unit_is_a_new_entity() {
    let mut w = World::new("s");
    let old = w
        .observe(&unit(
            5,
            "Red CAP 1-1",
            "Su-27",
            "Red CAP 1",
            Coalition::Red,
            Category::Airplane,
        ))
        .eid();
    w.mark_dead(old);
    // DCS respawns a unit with the same name, type and runtime id.
    let new = w.observe(&unit(
        5,
        "Red CAP 1-1",
        "Su-27",
        "Red CAP 1",
        Coalition::Red,
        Category::Airplane,
    ));
    assert!(matches!(new, Observed::New(_)), "{new:?}");
    assert_ne!(new.eid(), old);
    assert!(w.entity(old).unwrap().dead, "the old entity stays dead");
    assert!(w.entity(new.eid()).unwrap().is_alive());
    assert_eq!(w.entity_by_runtime_id(5).unwrap().eid, new.eid());
    assert_eq!(w.entity_by_name("Red CAP 1-1").unwrap().eid, new.eid());
}

#[test]
fn participants_resolve_to_dead_entities_without_ghosts() {
    let mut w = World::new("s");
    let a = w
        .observe(&unit(
            7,
            "A",
            "T-72B",
            "G",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid();
    w.mark_dead(a);
    let n = w.len();
    // A KILL event after DEAD names the same object.
    let p = w.observe_participant(&unit(
        7,
        "A",
        "T-72B",
        "G",
        Coalition::Red,
        Category::GroundUnit,
    ));
    assert_eq!(p, Observed::Same(a));
    assert!(w.entity(a).unwrap().dead);
    assert_eq!(w.len(), n, "no ghost entity");
    // A participant whose id_ could not be read is found by name.
    let by_name = w.observe_participant(&Observation {
        name: Some("A".into()),
        ..Observation::default()
    });
    assert_eq!(by_name, Observed::Same(a));
    assert_eq!(
        w.lookup(&Observation {
            runtime_id: Some(7),
            ..Observation::default()
        }),
        Some(a)
    );
}

#[test]
fn a_live_unit_seen_without_runtime_id_keeps_its_eid() {
    let mut w = World::new("s");
    let a = w
        .observe(&unit(
            3,
            "A",
            "T-72B",
            "G",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid();
    let again = w.observe(&Observation {
        name: Some("A".into()),
        type_name: Some("T-72B".into()),
        ..Observation::default()
    });
    assert_eq!(again, Observed::Same(a));
    assert!(w.entity(a).unwrap().is_alive());
}

#[test]
fn unchanged_observations_do_not_bump_the_revision() {
    let mut w = World::new("s");
    w.observe_group("G", None, Some(Coalition::Red), None, None);
    let o = unit(1, "A", "T-72B", "G", Coalition::Red, Category::GroundUnit);
    w.observe(&o);
    let rev = w.rev();
    w.observe(&o);
    w.observe_group("G", None, Some(Coalition::Red), None, None);
    assert_eq!(w.rev(), rev);
    assert!(!w.changed_since(rev));
    let mut moved = o.clone();
    moved.pos = Some(Vec3::new(1.0, 2.0, 3.0));
    w.observe(&moved);
    assert_eq!(w.rev(), rev + 1);
    assert_eq!(w.delta_since(rev).upserts.len(), 1);
}

#[cfg(feature = "link")]
#[test]
fn samples_update_positions_and_full_sweeps_retire_missing_units() {
    let mut w = World::new("s");
    let a = w
        .observe(&unit(
            1,
            "A",
            "F-16C_50",
            "G",
            Coalition::Blue,
            Category::Airplane,
        ))
        .eid();
    let b = w
        .observe(&unit(
            2,
            "B",
            "F-16C_50",
            "G",
            Coalition::Blue,
            Category::Airplane,
        ))
        .eid();
    let c = w
        .observe(&unit(
            3,
            "C",
            "F-16C_50",
            "G",
            Coalition::Blue,
            Category::Airplane,
        ))
        .eid();
    let statik = w
        .observe(&Observation {
            runtime_id: Some(4),
            name: Some("Hangar".into()),
            category: Some(Category::Static),
            ..Observation::default()
        })
        .eid();
    w.mark_dead(c);
    let rev = w.rev();
    let sample = |rid: u32, north: f64, alive: bool| link::Sample {
        runtime_id: rid,
        pos: Some(v1::Vec3 {
            x: north,
            y: 1000.0,
            z: 0.0,
        }),
        velocity: None,
        heading_rad: 0.5,
        alive,
    };
    let first = w.apply_samples(&link::Samples {
        sweep: 1,
        full_sweep: true,
        sweep_end: false,
        model_time: 10.0,
        samples: vec![sample(1, 100.0, true), sample(77, 0.0, true)],
    });
    assert_eq!(first.updated, vec![a]);
    assert_eq!(first.unknown, vec![77]);
    assert_eq!(w.rev(), rev + 1, "one revision per frame");
    let last = w.apply_samples(&link::Samples {
        sweep: 1,
        full_sweep: true,
        sweep_end: true,
        model_time: 10.1,
        samples: vec![sample(3, 0.0, false)],
    });
    assert!(last.unknown.is_empty(), "a dead wreck is not unknown");
    assert_eq!(last.gone, vec![b], "B was missing from the full sweep");
    assert!(w.entity(b).unwrap().dead);
    assert!(
        w.entity(statik).unwrap().is_alive(),
        "statics are not sampled"
    );
    assert_eq!(w.entity(a).unwrap().pos.unwrap().x, 100.0);
    assert_eq!(w.model_time(), 10.1);

    // A sample of a living unit that says dead kills it; resampling a dead one does nothing.
    let r = w.apply_samples(&link::Samples {
        sweep: 2,
        samples: vec![sample(1, 100.0, false)],
        ..Default::default()
    });
    assert_eq!(r.died, vec![a]);
    let rev = w.rev();
    w.apply_sample(&sample(1, 500.0, true));
    assert!(w.entity(a).unwrap().dead);
    assert_eq!(w.rev(), rev);
}

#[test]
fn polygon_zones_groups_types_and_nearest() {
    let mut w = World::new("s");
    w.observe_group(
        "Blue 1",
        None,
        Some(Coalition::Blue),
        Some(Category::Airplane),
        None,
    );
    w.observe_group(
        "Blue 2",
        None,
        Some(Coalition::Blue),
        Some(Category::Helicopter),
        None,
    );
    let a = w
        .observe(&at(
            unit(
                1,
                "Blue 1-1",
                "F-16C_50",
                "Blue 1",
                Coalition::Blue,
                Category::Airplane,
            ),
            1000.0,
            1000.0,
        ))
        .eid();
    let b = w
        .observe(&at(
            unit(
                2,
                "Blue 1-2",
                "F-16C_50",
                "Blue 1",
                Coalition::Blue,
                Category::Airplane,
            ),
            9000.0,
            9000.0,
        ))
        .eid();
    let h = w
        .observe(&at(
            unit(
                3,
                "Blue 2-1",
                "UH-1H",
                "Blue 2",
                Coalition::Blue,
                Category::Helicopter,
            ),
            1500.0,
            500.0,
        ))
        .eid();
    w.set_zones(&[mpb::Zone {
        ident: "BOX".into(),
        name: "Box".into(),
        shape: Some(mpb::zone::Shape::Polygon(mpb::Polygon {
            points: vec![
                v1::MapPoint {
                    north: 0.0,
                    east: 0.0,
                },
                v1::MapPoint {
                    north: 0.0,
                    east: 2000.0,
                },
                v1::MapPoint {
                    north: 2000.0,
                    east: 2000.0,
                },
                v1::MapPoint {
                    north: 2000.0,
                    east: 0.0,
                },
            ],
        })),
    }]);
    let boxed = w.zone("BOX").unwrap().clone();
    let inside: Vec<Eid> = w.units().inside(&boxed).iter().map(|e| e.eid).collect();
    assert_eq!(inside, vec![a, h]);
    assert_eq!(w.units().planes().count_inside(&boxed), 1);
    assert_eq!(w.units().in_zone(&boxed).helos().eids(), vec![h]);
    assert_eq!(w.units().in_group("Blue 1").eids(), vec![a, b]);
    assert_eq!(w.units().in_group("Nobody").count(), 0);
    assert_eq!(w.units().of_type("UH-1H").eids(), vec![h]);
    assert_eq!(
        w.units()
            .nearest_to(MapPoint::new(8000.0, 8000.0))
            .map(|e| e.eid),
        Some(b)
    );
    assert_eq!(
        w.units()
            .within(MapPoint::new(9000.0, 9000.0), sortie_types::m(10))
            .eids(),
        vec![b]
    );
    assert!(w.group("Blue 1").any_inside(&boxed));
    assert!(!w.group("Blue 1").all_inside(&boxed));
    assert!(w.group("Blue 2").all_inside(&boxed));
    assert_eq!(
        w.group("Blue 1").center(),
        Some(MapPoint::new(5000.0, 5000.0))
    );
    // Moving a unit moves it in the grid.
    w.observe(&at(
        unit(
            2,
            "Blue 1-2",
            "F-16C_50",
            "Blue 1",
            Coalition::Blue,
            Category::Airplane,
        ),
        100.0,
        100.0,
    ));
    assert_eq!(w.units().in_zone(&boxed).eids(), vec![a, b, h]);
}

/// A polygon zone that is not in the manifest is copied into the query.
struct AdHoc(Vec<MapPoint>);

impl AsZone for AdHoc {
    fn zone_ident(&self) -> &str {
        "AD_HOC"
    }

    fn zone_shape(&self) -> ZoneShape<'_> {
        ZoneShape::Polygon { points: &self.0 }
    }
}

#[test]
fn ad_hoc_zones_statics_and_players() {
    let mut w = World::new("s");
    let tank = w
        .observe(&at(
            unit(1, "T", "T-72B", "G", Coalition::Red, Category::GroundUnit),
            5.0,
            5.0,
        ))
        .eid();
    let hangar = w
        .observe(&Observation {
            runtime_id: Some(2),
            name: Some("Hangar".into()),
            category: Some(Category::Static),
            pos: Some(Vec3::new(5.0, 0.0, 6.0)),
            ..Observation::default()
        })
        .eid();
    let mut viper = unit(3, "V", "F-16C_50", "B", Coalition::Blue, Category::Airplane);
    viper.player = Some("Maverick".into());
    let viper = w.observe(&viper).eid();
    let zone = AdHoc(vec![
        MapPoint::new(0.0, 0.0),
        MapPoint::new(0.0, 10.0),
        MapPoint::new(10.0, 10.0),
        MapPoint::new(10.0, 0.0),
    ]);
    let q = w.units().in_zone(&zone);
    drop(zone);
    assert_eq!(q.eids(), vec![tank]);
    assert_eq!(w.statics().eids(), vec![hangar]);
    assert_eq!(w.units().any_category().count(), 3);
    assert_eq!(w.players().eids(), vec![viper]);
    assert!(w.units().dead().eids().is_empty());
}

#[test]
fn respawned_group_inherits_the_manifest_ident() {
    const SAM: GroupRef<Op, Ground> = GroupRef::new("SAM_NORTH");
    let mut w = World::new("s");
    let g1 = w.observe_group(
        "SAM North",
        Some("SAM_NORTH"),
        Some(Coalition::Red),
        None,
        Some(1),
    );
    let u = w
        .observe(&unit(
            1,
            "SAM North-1",
            "S-300PS 54K6 cp",
            "SAM North",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid();
    w.mark_dead(u);
    assert!(w.group(&SAM).is_dead());
    let g2 = w.observe_group("SAM North", None, None, None, Some(9));
    assert_ne!(g1, g2);
    assert_eq!(
        w.group(&SAM).eid(),
        Some(g2),
        "the symbol follows the respawn"
    );
    assert_eq!(w.group(&g2).ident(), Some("SAM_NORTH"));
    assert_eq!(w.group(&g2).coalition(), Some(Coalition::Red));
    assert!(
        w.group(&g1).is_dead(),
        "the old handle still sees the old group"
    );
    assert!(!w.group(&g2).is_dead(), "a group without units is not dead");
}

#[test]
fn airbases_have_deltas_and_change_owner() {
    let mut w = World::new("s");
    let base = w.rev();
    let ab = w.upsert_airbase(v1::AirbaseSnap {
        name: "Maykop-Khanskaya".into(),
        ident: "MAYKOP_KHANSKAYA".into(),
        coalition: Coalition::Red.as_i32(),
        airbase_id: 16,
        ..Default::default()
    });
    assert_eq!(w.airbase_by_ident("MAYKOP_KHANSKAYA").unwrap().eid, ab);
    let mut replica = World::replica("s");
    replica.apply_delta(&w.delta_since(base)).unwrap();
    assert_eq!(
        replica
            .airbase_by_name("Maykop-Khanskaya")
            .unwrap()
            .airbase_id,
        Some(16)
    );

    let before = w.rev();
    let snap = w.airbase(ab).unwrap().to_snap();
    let ev = v1::Event {
        kind: Some(v1::event::Kind::BaseCaptured(v1::BaseCaptured {
            initiator: None,
            place: Some(snap),
            coalition: Coalition::Blue.as_i32(),
        })),
        ..Default::default()
    };
    w.apply_event(&ev);
    assert_eq!(w.airbase(ab).unwrap().coalition, Some(Coalition::Blue));
    let d = w.delta_since(before);
    assert_eq!(d.airbase_upserts.len(), 1);
    replica.apply_event(&ev);
    replica.apply_delta(&d).unwrap();
    assert_eq!(replica.snapshot(), w.snapshot());
}

/// The owner event of wave 2 (event revision 2): a replica takes the new owner from it before
/// user code sees it, and learns an airbase it did not know from its snap. The revision never
/// moves on a replica.
#[test]
fn an_owner_change_gives_the_replica_the_new_owner() {
    let mut w = World::new("s");
    let base = w.rev();
    let ab = w.upsert_airbase(v1::AirbaseSnap {
        name: "Kutaisi".into(),
        coalition: Coalition::Blue.as_i32(),
        airbase_id: 25,
        ..Default::default()
    });
    let mut replica = World::replica("s");
    replica.apply_delta(&w.delta_since(base)).unwrap();
    let rev = replica.rev();
    let changed = |snap: v1::AirbaseSnap, owner: Coalition| v1::Event {
        kind: Some(v1::event::Kind::AirbaseOwnerChanged(
            v1::AirbaseOwnerChanged {
                airbase: Some(snap),
                previous: v1::Coalition::Blue as i32,
                owner: owner.as_i32(),
                cause: v1::OwnerChangeCause::Control as i32,
            },
        )),
        ..Default::default()
    };
    // The snap is the base after the change; the owner field is what counts.
    let snap = w.airbase(ab).unwrap().to_snap();
    replica.apply_event(&changed(snap, Coalition::Red));
    assert_eq!(replica.airbase(ab).unwrap().coalition, Some(Coalition::Red));
    assert_eq!(replica.rev(), rev, "a replica's revision does not move");
    // An airbase the replica did not hold comes with its snap.
    let farp = v1::AirbaseSnap {
        eid: sortie_proto::eid::pack(900, 1),
        name: "FARP North".into(),
        coalition: Coalition::Red.as_i32(),
        ..Default::default()
    };
    replica.apply_event(&changed(farp, Coalition::Red));
    assert_eq!(
        replica.airbase_by_name("FARP North").map(|a| a.coalition),
        Some(Some(Coalition::Red))
    );
}

#[test]
fn pruning_tombstones_and_history() {
    let mut w = World::new("s");
    w.observe_group("Red CAP#7XQ2-1", None, Some(Coalition::Red), None, None);
    let u = w
        .observe(&unit(
            1,
            "Red CAP#7XQ2-1-1",
            "Su-27",
            "Red CAP#7XQ2-1",
            Coalition::Red,
            Category::Airplane,
        ))
        .eid();
    let g = w.group("Red CAP#7XQ2-1").eid().unwrap();
    let mut replica = World::from_snapshot(&w.snapshot());
    w.set_model_time(100.0);
    w.mark_dead(u);
    assert!(w.prune_dead(60.0).is_empty(), "too recent");
    w.set_model_time(200.0);
    let from = replica.rev();
    assert_eq!(w.prune_dead(60.0), vec![u]);
    assert!(w.entity(u).is_none());
    assert!(
        w.group(&g).entry().is_none(),
        "an emptied runtime group goes too"
    );
    let d = w.try_delta_since(from).unwrap();
    assert_eq!(d.removals, vec![u.to_u64()]);
    assert_eq!(d.group_removals, vec![g.to_u64()]);
    replica.apply_delta(&d).unwrap();
    assert_eq!(replica.snapshot(), w.snapshot());

    w.forget_history_through(w.rev());
    assert!(matches!(
        w.try_delta_since(from),
        Err(WorldError::HistoryPruned { .. })
    ));
    assert!(w.try_delta_since(w.rev()).is_ok());
    assert!(matches!(
        w.try_delta_since(w.rev() + 1),
        Err(WorldError::RevisionMismatch { .. })
    ));
}

#[test]
fn manifest_groups_survive_pruning_and_count_pruned_units_as_dead() {
    let mut w = World::new("s");
    let g = w.observe_group("SAM North", Some("SAM_NORTH"), None, None, None);
    let u = w
        .observe(&unit(
            1,
            "SAM North-1",
            "S-300PS 54K6 cp",
            "SAM North",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid();
    w.mark_dead(u);
    w.prune_dead(0.0);
    assert!(w.group(&g).exists());
    assert!(w.group(&g).is_dead());
    assert_eq!(w.group(&g).alive_count(), 0);
}

#[test]
fn diff_and_track() {
    let mut a = World::new("s");
    a.observe_group("G", None, None, None, None);
    let x = a
        .observe(&unit(
            1,
            "X",
            "T-72B",
            "G",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid();
    let mut b = a.clone();
    let (y, tracked) = b.track(|w| {
        w.mark_dead(x);
        w.observe(&unit(
            2,
            "Y",
            "T-72B",
            "G",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid()
    });
    let d = a.diff(&b);
    assert_eq!(d.from_rev, a.rev());
    assert_eq!(d.rev, b.rev());
    let mut ups: Vec<u64> = d.upserts.iter().map(|s| s.eid).collect();
    ups.sort_unstable();
    assert_eq!(ups, vec![x.to_u64(), y.to_u64()]);
    assert_eq!(tracked.upserts.len(), 2);
    let mut r = World::from_snapshot(&a.snapshot());
    r.apply_delta(&d).unwrap();
    assert_eq!(r.snapshot(), b.snapshot());
    assert!(a.diff(&a).upserts.is_empty());
}

#[test]
fn names_resolve_to_the_newest_object() {
    let mut w = World::new("s");
    let a = w
        .observe(&unit(
            1,
            "A",
            "T-72B",
            "G",
            Coalition::Red,
            Category::GroundUnit,
        ))
        .eid();
    // Same name, other type: DCS replaced the object.
    let b = w.observe(&unit(
        2,
        "A",
        "T-80U",
        "G",
        Coalition::Red,
        Category::GroundUnit,
    ));
    assert_eq!(b.retired(), Some(a));
    assert_eq!(w.entity_by_name("A").unwrap().eid, b.eid());
    // A replica rebuilt from a snapshot resolves names the same way.
    let r = World::from_snapshot(&w.snapshot());
    assert_eq!(r.entity_by_name("A").unwrap().eid, b.eid());
    w.remove(b.eid());
    assert_eq!(
        w.entity_by_name("A").unwrap().eid,
        a,
        "falls back to the older one"
    );
}

#[test]
fn airbase_hosts_deaths_and_removals_reach_replicas() {
    let mut w = World::new("s");
    let farp = w.observe(&Observation {
        runtime_id: Some(77),
        name: Some("FARP Dallas".into()),
        type_name: Some("FARP".into()),
        category: Some(Category::Static),
        ..Observation::default()
    });
    let base = w.rev();
    let ab = w.upsert_airbase(v1::AirbaseSnap {
        name: "FARP Dallas".into(),
        host_eid: farp.eid().to_u64(),
        ..Default::default()
    });
    let mut replica = World::replica("s");
    replica.apply_snapshot(&w.snapshot());
    let snap_rev = w.rev();
    assert_eq!(replica.airbase(ab).unwrap().host, Some(farp.eid()));

    // An update that does not know the host keeps it; death is final.
    w.upsert_airbase(v1::AirbaseSnap {
        name: "FARP Dallas".into(),
        ..Default::default()
    });
    assert_eq!(w.airbase(ab).unwrap().host, Some(farp.eid()));
    assert!(w.mark_airbase_dead(ab));
    assert!(!w.mark_airbase_dead(ab));
    w.upsert_airbase(v1::AirbaseSnap {
        name: "FARP Dallas".into(),
        dead: false,
        ..Default::default()
    });
    assert!(w.airbase(ab).unwrap().dead);

    replica.apply_delta(&w.delta_since(snap_rev)).unwrap();
    assert!(replica.airbase(ab).unwrap().dead);

    let before = w.rev();
    w.remove_airbase(ab).unwrap();
    assert!(w.airbase_by_name("FARP Dallas").is_none());
    let d = w.delta_since(before);
    assert_eq!(d.airbase_removals, vec![ab.to_u64()]);
    replica.apply_delta(&d).unwrap();
    assert!(replica.airbase(ab).is_none());
    assert!(replica.airbase_by_name("FARP Dallas").is_none());
    assert_eq!(w.delta_since(base).airbase_removals, vec![ab.to_u64()]);

    // A diff sees the removal too.
    let mut with = World::new("s");
    let x = with.upsert_airbase(v1::AirbaseSnap {
        name: "Ship deck".into(),
        ..Default::default()
    });
    let mut without = with.clone();
    without.remove_airbase(x);
    assert_eq!(with.diff(&without).airbase_removals, vec![x.to_u64()]);
}

#[test]
fn reply_snaps_apply_as_observations_until_the_replica_catches_up() {
    let mut mirror = World::new("s");
    let mut replica = World::replica("s");
    replica.apply_snapshot(&mirror.snapshot());
    let g = mirror.observe_group(
        "Red CAP#ab12-1",
        None,
        Some(Coalition::Red),
        Some(Category::Airplane),
        Some(5),
    );
    let u = mirror
        .observe(&unit(
            31,
            "Red CAP#ab12-1-1",
            "MiG-29S",
            "Red CAP#ab12-1",
            Coalition::Red,
            Category::Airplane,
        ))
        .eid();
    let reply = v1::Reply {
        groups: vec![mirror.group_entry(GroupKey::Eid(g)).unwrap().to_snap()],
        entities: vec![mirror.entity(u).unwrap().to_snap()],
        ..v1::Reply::default()
    };
    let world_rev = mirror.rev();
    let rev = replica.rev();
    assert!(replica.apply_reply(&reply, world_rev));
    assert_eq!(replica.rev(), rev, "a reply never moves the revision");
    assert_eq!(replica.entity(u).unwrap().name, "Red CAP#ab12-1-1");
    assert_eq!(
        replica.group_entry(GroupKey::Eid(g)).unwrap().units,
        vec![u]
    );
    // The delta that carries the same facts still applies.
    replica.apply_delta(&mirror.delta_since(rev)).unwrap();
    assert_eq!(replica.rev(), world_rev);

    // An older reply cannot overwrite newer state.
    mirror.mark_dead(u);
    replica.apply_delta(&mirror.delta_since(world_rev)).unwrap();
    assert!(!replica.apply_reply(&reply, world_rev));
    assert!(replica.entity(u).unwrap().dead);
    // The mirror itself ignores replies; world_rev 0 means "touched nothing".
    assert!(!mirror.apply_reply(&reply, mirror.rev() + 1));
    let mut fresh = World::replica("s");
    assert!(!fresh.apply_reply(&reply, 0));
}

fn drawing(name: &str, drifted: bool) -> v1::ResourceSnap {
    v1::ResourceSnap {
        kind: v1::ResourceKind::Drawing as i32,
        name: name.into(),
        phase: v1::ResourcePhase::Active as i32,
        owner: "w".into(),
        drifted,
        ..v1::ResourceSnap::default()
    }
}

#[test]
fn the_registry_travels_in_snapshots_deltas_diffs_and_replies() {
    let mut mirror = World::new("s");
    let before = mirror.clone();
    assert!(mirror.put_resource(drawing("b", false)));
    assert!(mirror.put_resource(drawing("a", false)));
    assert!(
        !mirror.put_resource(drawing("a", false)),
        "equal: no change"
    );
    let r1 = mirror.rev();
    assert_eq!(r1, 2);
    let mut replica = World::from_snapshot(&mirror.snapshot());
    let names: Vec<_> = replica.resources().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["a", "b"], "by kind, then name");

    // A removal and a put of the same name again: removals first, then upserts.
    mirror.remove_resource(v1::ResourceKind::Drawing, "a");
    mirror.put_resource(drawing("a", true));
    mirror.remove_resource(v1::ResourceKind::Drawing, "b");
    let d = mirror.delta_since(r1);
    assert_eq!(d.resource_upserts.len(), 1);
    assert_eq!(d.resource_removals.len(), 2);
    replica.apply_delta(&d).unwrap();
    let a = replica.resource(v1::ResourceKind::Drawing, "a").unwrap();
    assert!(a.drifted);
    assert!(replica.resource(v1::ResourceKind::Drawing, "b").is_none());
    assert_eq!(replica.resource_count(), 1);

    // A diff needs no history.
    let diff = before.diff(&mirror);
    assert_eq!(diff.resource_upserts.len(), 1);
    let mut from_scratch = before.clone();
    from_scratch.apply_delta(&diff).unwrap();
    assert_eq!(from_scratch.resource_count(), 1);

    // A command's reply shows the resource before the next delta.
    let reply = v1::Reply {
        resource: Some(v1::ResourceAck {
            resource: Some(drawing("c", false)),
            ..v1::ResourceAck::default()
        }),
        ..v1::Reply::default()
    };
    assert!(replica.apply_reply(&reply, replica.rev() + 1));
    assert!(replica.resource(v1::ResourceKind::Drawing, "c").is_some());

    // Forgotten history: the removal log goes with it.
    mirror.forget_history_through(mirror.rev());
    assert!(
        mirror
            .delta_since(mirror.rev())
            .resource_removals
            .is_empty()
    );
}

/// Weapons and crew are named in the identity table, never as entities: the Eids come from the
/// same allocator, the revision does not move, snapshots and deltas do not carry them.
#[test]
fn identities_share_the_allocator_and_stay_out_of_the_world() {
    let mut w = World::new("s");
    let u = w
        .observe(&unit(
            10,
            "Viper 1-1",
            "F-16C_50",
            "Blue 1",
            Coalition::Blue,
            Category::Airplane,
        ))
        .eid();
    let rev = w.rev();
    let missile = w.identify_weapon(500, Some("AIM_120C"), false);
    assert_ne!(missile, u);
    assert_eq!(
        w.identify_weapon(500, None, true),
        missile,
        "a gone weapon: id_ only"
    );
    assert_eq!(w.identify_weapon(500, Some("AIM_120C"), true), missile);
    let pilot = w.identify_crew(600, None);
    assert_eq!(w.identify_crew(600, Some("pilot")), pilot);
    assert_eq!(w.identity(pilot).unwrap().type_name, "pilot");
    assert_eq!((w.rev(), w.len()), (rev, 1));
    assert!(w.snapshot().entities.iter().all(|e| e.eid == u.to_u64()));
    assert!(w.delta_since(0).upserts.iter().all(|e| e.eid == u.to_u64()));
    // A crew member's runtime id now names a weapon: the crew identity ends.
    let other = w.identify_weapon(600, Some("Mk_82"), false);
    assert_ne!(other, pilot);
    assert!(!w.identity(pilot).unwrap().is_live(0.0));
    assert!(w.end_identity(other));
    assert!(!w.end_identity(other), "ended once");
    assert!(w.identity_by_runtime_id(600).is_none());
    // The weapon expires after the strike window; crew never expire by time.
    let crew = w.identify_crew(700, None);
    w.set_model_time(WEAPON_AFTER_STRIKE_S + 1.0);
    assert!(w.identity_by_runtime_id(500).is_none());
    assert!(w.identity_by_runtime_id(700).is_some_and(|i| i.eid == crew));
}

/// Review C5: a SHOT names a weapon at the start of its life: it never extends an older live
/// identity at the same runtime id and type; the older one ends.
#[test]
fn a_fired_weapon_always_gets_a_new_identity() {
    let mut w = World::new("s");
    let a = w.identify_weapon(5000, Some("AIM_9"), false);
    w.set_model_time(60.0);
    let b = w.identify_weapon_fired(5000, Some("AIM_9"));
    assert_ne!(a, b);
    assert!(w.identity(a).unwrap().ended);
    assert_eq!(w.identity_by_runtime_id(5000).unwrap().eid, b);
    assert_eq!(w.identify_weapon(5000, Some("AIM_9"), true), b);
}

/// Reviews C3, D4: a mirror entity that takes a runtime id (DCS reused a pilot's `id_`) ends the
/// identity there: the old crew Eid no longer selects the new holder.
#[test]
fn a_mirror_entity_taking_a_runtime_id_ends_its_identity() {
    let mut w = World::new("s");
    let pilot = w.identify_crew(600, None);
    let missile = w.identify_weapon(601, Some("AIM_9"), false);
    w.observe(&unit(
        600,
        "Hawk 1-1",
        "F-16C_50",
        "Hawk 1",
        Coalition::Blue,
        Category::Airplane,
    ));
    assert!(w.identity_by_runtime_id(600).is_none());
    assert!(w.identity(pilot).unwrap().ended);
    w.observe_participant(&unit(
        601,
        "Hawk 1-2",
        "F-16C_50",
        "Hawk 1",
        Coalition::Blue,
        Category::Airplane,
    ));
    assert!(w.identity(missile).unwrap().ended);
}

/// A non-finite model time is ignored (it drives identity lifetimes; `inf - inf` is NaN).
#[test]
fn a_non_finite_model_time_is_ignored() {
    let mut w = World::new("s");
    w.set_model_time(12.0);
    w.set_model_time(f64::INFINITY);
    w.set_model_time(f64::NAN);
    assert_eq!(w.model_time(), 12.0);
}

/// The table is bounded: ended entries go first, then the ones named longest ago.
#[test]
fn the_identity_table_is_bounded() {
    let mut w = World::new("s");
    for rid in 0..u32::try_from(MAX_IDENTITIES).unwrap() {
        w.set_model_time(f64::from(rid) * 0.001);
        w.identify_crew(rid, None);
    }
    assert_eq!(w.identity_count(), MAX_IDENTITIES);
    let first = w.identity_by_runtime_id(0).unwrap().eid;
    w.identify_crew(1_000_000, None);
    assert!(w.identity_count() <= MAX_IDENTITIES * 3 / 4 + 1);
    assert!(w.identity(first).is_none(), "the oldest went first");
    assert!(w.identity_by_runtime_id(1_000_000).is_some());
}

/// A replica adds no weapon and no nameless object it does not know from an event (the engine
/// names them in its identity table only).
#[test]
fn a_replica_adds_no_weapon_or_crew_from_events() {
    let mut r = World::replica("s");
    let snap = |eid: u64, name: &str, category: Category| v1::EntitySnap {
        eid,
        name: name.into(),
        category: category.as_i32(),
        ..v1::EntitySnap::default()
    };
    let one = Eid::from_u64(1 << 32 | 1).unwrap().to_u64();
    let three = Eid::from_u64(1 << 32 | 3).unwrap().to_u64();
    let seven = Eid::from_u64(1 << 32 | 7).unwrap().to_u64();
    let hit = v1::Event {
        kind: Some(v1::event::Kind::Hit(v1::Hit {
            initiator: Some(snap(one, "Viper 1-1", Category::Airplane)),
            target: Some(snap(three, "", Category::Static)),
            weapon: Some(v1::WeaponSnap {
                eid: seven,
                ..v1::WeaponSnap::default()
            }),
        })),
        ..v1::Event::default()
    };
    r.apply_event(&hit);
    let kill = v1::Event {
        kind: Some(v1::event::Kind::Kill(v1::Kill {
            initiator: Some(snap(one, "Viper 1-1", Category::Airplane)),
            target: Some(snap(seven, "R-73", Category::Weapon)),
            weapon: None,
        })),
        ..v1::Event::default()
    };
    r.apply_event(&kill);
    let eids: Vec<u64> = r.entities().map(|e| e.eid.to_u64()).collect();
    assert_eq!(eids, [one]);
}
