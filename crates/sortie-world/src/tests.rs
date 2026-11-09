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
