//! Property tests of the world model:
//!
//! * applying `diff(a, b)` (or `b.delta_since(a.rev())`) to a replica of `a` yields `b`;
//! * a replica that follows the mirror tick by tick (events first, then the delta) stays equal;
//! * runtime-id reuse never resurrects a dead Eid, and a runtime id has at most one live owner;
//! * zone queries through the spatial grid agree with a brute-force scan.
//!
//! The operations include link samples and sweeps, so the file needs the engine-side feature
//! `link` (`cargo test --workspace` enables it through the engine).

#![cfg(feature = "link")]

use std::collections::{BTreeSet, HashMap};

use proptest::prelude::*;
use sortie_proto::link::v1 as link;
use sortie_proto::v1;
use sortie_world::{
    AsZone, Category, Coalition, Eid, MapPoint, Observation, Observed, Vec3, World, ZoneShape,
};

const NAMES: &[&str] = &["A", "B", "C", "D", "E"];
const TYPES: &[&str] = &["T-72B", "Su-27"];
const GROUPS: &[&str] = &["G1", "G2", "G3"];

#[derive(Clone, Debug)]
enum Op {
    Group(usize),
    Observe {
        rid: Option<u32>,
        name: usize,
        ty: usize,
        group: usize,
        pos: (i32, i32),
        participant: bool,
    },
    MarkDead(usize),
    Sample {
        rid: u32,
        pos: (i32, i32),
        alive: bool,
    },
    Sweep {
        id: u64,
        end: bool,
        rids: Vec<u32>,
    },
    Remove(usize),
    Advance(u8),
    Prune(u8),
    Airbase(usize, bool),
    DeadEvent(usize),
}

fn op() -> impl Strategy<Value = Op> {
    let pos = (-30i32..30, -30i32..30);
    prop_oneof![
        (0..GROUPS.len()).prop_map(Op::Group),
        (
            prop::option::weighted(0.9, 1u32..6),
            0..NAMES.len(),
            0..TYPES.len(),
            0..GROUPS.len(),
            pos.clone(),
            any::<bool>()
        )
            .prop_map(|(rid, name, ty, group, pos, participant)| Op::Observe {
                rid,
                name,
                ty,
                group,
                pos,
                participant
            }),
        any::<usize>().prop_map(Op::MarkDead),
        (1u32..6, pos, any::<bool>()).prop_map(|(rid, pos, alive)| Op::Sample { rid, pos, alive }),
        (1u64..3, any::<bool>(), prop::collection::vec(1u32..6, 0..4))
            .prop_map(|(id, end, rids)| Op::Sweep { id, end, rids }),
        any::<usize>().prop_map(Op::Remove),
        any::<u8>().prop_map(Op::Advance),
        (0u8..3).prop_map(Op::Prune),
        (0..2usize, any::<bool>()).prop_map(|(i, blue)| Op::Airbase(i, blue)),
        any::<usize>().prop_map(Op::DeadEvent),
    ]
}

fn vec3((n, e): (i32, i32)) -> Vec3 {
    Vec3::new(f64::from(n) * 700.0, 100.0, f64::from(e) * 700.0)
}

fn pick(w: &World, i: usize) -> Option<Eid> {
    let all: Vec<Eid> = w.entities().map(|e| e.eid).collect();
    (!all.is_empty()).then(|| all[i % all.len()])
}

/// Applies `op` to the mirror; returns the event the engine would send for it, if any.
fn apply(w: &mut World, op: &Op) -> Option<v1::Event> {
    match op {
        Op::Group(g) => {
            w.observe_group(GROUPS[*g], None, Some(Coalition::Red), None, None);
        }
        Op::Observe {
            rid,
            name,
            ty,
            group,
            pos,
            participant,
        } => {
            let obs = Observation {
                runtime_id: *rid,
                name: Some(NAMES[*name].into()),
                type_name: Some(TYPES[*ty].into()),
                coalition: Some(if *ty == 0 {
                    Coalition::Red
                } else {
                    Coalition::Blue
                }),
                category: Some(if *ty == 0 {
                    Category::GroundUnit
                } else {
                    Category::Airplane
                }),
                group_name: Some(GROUPS[*group].into()),
                pos: Some(vec3(*pos)),
                ..Observation::default()
            };
            let o = if *participant {
                w.observe_participant(&obs)
            } else {
                w.observe(&obs)
            };
            let e = w.entity(o.eid()).expect("observed entity exists");
            if !participant {
                assert!(e.is_alive(), "a live observation yields a live entity");
                let snap = e.to_snap();
                return Some(v1::Event {
                    kind: Some(v1::event::Kind::Birth(v1::Birth {
                        initiator: Some(snap),
                        place: None,
                    })),
                    ..Default::default()
                });
            }
        }
        Op::MarkDead(i) => {
            if let Some(e) = pick(w, *i) {
                w.mark_dead(e);
            }
        }
        Op::Sample { rid, pos, alive } => {
            w.apply_sample(&link::Sample {
                runtime_id: *rid,
                pos: Some(vec3(*pos).into()),
                velocity: None,
                heading_rad: 1.0,
                alive: *alive,
            });
        }
        Op::Sweep { id, end, rids } => {
            w.apply_samples(&link::Samples {
                sweep: *id,
                full_sweep: true,
                sweep_end: *end,
                model_time: w.model_time(),
                samples: rids
                    .iter()
                    .map(|r| link::Sample {
                        runtime_id: *r,
                        pos: None,
                        velocity: None,
                        heading_rad: 0.0,
                        alive: true,
                    })
                    .collect(),
            });
        }
        Op::Remove(i) => {
            if let Some(e) = pick(w, *i) {
                w.remove(e);
            }
        }
        Op::Advance(dt) => {
            let t = w.model_time() + f64::from(*dt);
            w.set_model_time(t);
        }
        Op::Prune(k) => {
            w.prune_dead(f64::from(*k) * 50.0);
        }
        Op::Airbase(i, blue) => {
            w.upsert_airbase(v1::AirbaseSnap {
                name: ["Maykop-Khanskaya", "Senaki-Kolkhi"][*i].into(),
                coalition: if *blue {
                    Coalition::Blue.as_i32()
                } else {
                    Coalition::Red.as_i32()
                },
                ..Default::default()
            });
        }
        Op::DeadEvent(i) => {
            let e = pick(w, *i)?;
            w.mark_dead(e);
            let snap = w.entity(e)?.to_snap();
            let ev = v1::Event {
                kind: Some(v1::event::Kind::Dead(v1::Dead {
                    initiator: Some(snap),
                })),
                ..Default::default()
            };
            // Applying the decoded event to the mirror is harmless.
            let rev = w.rev();
            w.apply_event(&ev);
            assert_eq!(
                w.rev(),
                rev,
                "re-applying the mirror's own facts changes nothing"
            );
            return Some(ev);
        }
    }
    None
}

fn world_from(ops: &[Op]) -> World {
    let mut w = World::new("01SORTIE000000000000000001");
    for o in ops {
        apply(&mut w, o);
    }
    w
}

/// Everything a replica must agree on besides the snapshot: name and ident lookups.
fn assert_same_lookups(r: &World, m: &World) {
    for n in NAMES {
        assert_eq!(
            r.entity_by_name(n).map(|e| e.eid),
            m.entity_by_name(n).map(|e| e.eid),
            "entity_by_name({n})"
        );
    }
    for g in GROUPS {
        assert_eq!(r.group(*g).eid(), m.group(*g).eid(), "group({g})");
        assert_eq!(r.group(*g).alive_count(), m.group(*g).alive_count());
    }
    assert_eq!(r.units().eids(), m.units().eids());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn diff_of_a_and_b_applied_to_a_yields_b(
        first in prop::collection::vec(op(), 0..30),
        second in prop::collection::vec(op(), 0..30),
    ) {
        let a = world_from(&first);
        let mut b = a.clone();
        for o in &second {
            apply(&mut b, o);
        }
        let mut r = World::from_snapshot(&a.snapshot());
        r.apply_delta(&a.diff(&b)).unwrap();
        prop_assert_eq!(r.snapshot(), b.snapshot());
        assert_same_lookups(&r, &b);

        // The incremental delta says the same thing.
        let mut r2 = World::from_snapshot(&a.snapshot());
        r2.apply_delta(&b.delta_since(a.rev())).unwrap();
        prop_assert_eq!(r2.snapshot(), b.snapshot());
        assert_same_lookups(&r2, &b);
    }

    #[test]
    fn a_replica_following_events_and_deltas_stays_in_sync(
        ops in prop::collection::vec(op(), 0..60),
    ) {
        let mut m = World::new("s");
        let mut r = World::from_snapshot(&m.snapshot());
        for o in &ops {
            let ev = apply(&mut m, o);
            if let Some(ev) = ev {
                let rev = r.rev();
                r.apply_event(&ev);
                prop_assert_eq!(r.rev(), rev);
            }
            r.apply_delta(&m.delta_since(r.rev())).unwrap();
            prop_assert_eq!(r.snapshot(), m.snapshot());
        }
        assert_same_lookups(&r, &m);
    }

    #[test]
    fn runtime_id_reuse_never_resurrects(ops in prop::collection::vec(op(), 0..80)) {
        let mut w = World::new("s");
        let mut dead_ever: BTreeSet<Eid> = BTreeSet::new();
        let mut ever: BTreeSet<Eid> = BTreeSet::new();
        for o in &ops {
            let before: BTreeSet<Eid> = w.entities().map(|e| e.eid).collect();
            apply(&mut w, o);
            for e in w.entities() {
                if dead_ever.contains(&e.eid) {
                    prop_assert!(e.dead, "{} came back to life after {:?}", e.eid, o);
                }
                if e.dead {
                    dead_ever.insert(e.eid);
                }
            }
            // At most one live entity per runtime id.
            let mut owners: HashMap<u32, Eid> = HashMap::new();
            for e in w.units().any_category().iter() {
                if let Some(rid) = e.runtime_id {
                    prop_assert!(owners.insert(rid, e.eid).is_none(), "two live owners of id {}", rid);
                }
            }
            // A new Eid is never one that existed before (removed or not).
            for e in w.entities() {
                if !before.contains(&e.eid) {
                    prop_assert!(ever.insert(e.eid), "Eid {} reused", e.eid);
                }
            }
        }
    }

    #[test]
    fn birth_after_death_always_allocates(
        rid in 1u32..4,
        name in 0..NAMES.len(),
        ty in 0..TYPES.len(),
    ) {
        let mut w = World::new("s");
        let obs = Observation {
            runtime_id: Some(rid),
            name: Some(NAMES[name].into()),
            type_name: Some(TYPES[ty].into()),
            category: Some(Category::Airplane),
            ..Observation::default()
        };
        let first = w.observe(&obs).eid();
        w.mark_dead(first);
        let second = w.observe(&obs);
        prop_assert!(matches!(second, Observed::New(_)));
        prop_assert_ne!(second.eid(), first);
        prop_assert!(w.entity(first).unwrap().dead);
        // A participant observation of the same object finds the live successor.
        prop_assert_eq!(w.observe_participant(&obs).eid(), second.eid());
    }

    #[test]
    fn grid_queries_agree_with_a_scan(
        points in prop::collection::vec((-200i32..200, -200i32..200, any::<bool>()), 0..60),
        zone in zone_strategy(),
    ) {
        let mut w = World::new("s");
        for (i, (n, e, dead)) in points.iter().enumerate() {
            let eid = w
                .observe(&Observation {
                    runtime_id: Some(i as u32 + 1),
                    name: Some(format!("U{i}")),
                    category: Some(Category::GroundUnit),
                    pos: Some(Vec3::new(f64::from(*n) * 450.0, 0.0, f64::from(*e) * 450.0)),
                    ..Observation::default()
                })
                .eid();
            if *dead {
                w.mark_dead(eid);
            }
        }
        let z = TestZone(zone);
        let shape = z.zone_shape();
        let brute: Vec<Eid> = w
            .entities()
            .filter(|e| e.is_alive() && e.map_point().is_some_and(|p| shape.contains(p)))
            .map(|e| e.eid)
            .collect();
        prop_assert_eq!(w.units().in_zone(&z).eids(), brute.clone());
        let inside: Vec<Eid> = w.units().inside(&z).iter().map(|e| e.eid).collect();
        prop_assert_eq!(inside, brute.clone());
        prop_assert_eq!(w.units().count_inside(&z), brute.len());
        prop_assert_eq!(w.units().any_inside(&z), !brute.is_empty());
        let all: usize = w
            .entities()
            .filter(|e| e.map_point().is_some_and(|p| shape.contains(p)))
            .count();
        prop_assert_eq!(w.units().including_dead().count_inside(&z), all);
    }
}

#[derive(Clone, Debug)]
enum ZoneSpec {
    Circle(MapPoint, f64),
    Polygon(Vec<MapPoint>),
}

struct TestZone(ZoneSpec);

impl AsZone for TestZone {
    fn zone_ident(&self) -> &str {
        "TEST"
    }

    fn zone_shape(&self) -> ZoneShape<'_> {
        match &self.0 {
            ZoneSpec::Circle(c, r) => ZoneShape::Circle {
                center: *c,
                radius_m: *r,
            },
            ZoneSpec::Polygon(p) => ZoneShape::Polygon { points: p },
        }
    }
}

fn zone_strategy() -> impl Strategy<Value = ZoneSpec> {
    let point =
        (-100_000.0..100_000.0f64, -100_000.0..100_000.0f64).prop_map(|(n, e)| MapPoint::new(n, e));
    prop_oneof![
        (point.clone(), 1.0..80_000.0f64).prop_map(|(c, r)| ZoneSpec::Circle(c, r)),
        prop::collection::vec(point, 3..7).prop_map(ZoneSpec::Polygon),
    ]
}
