//! Validation rules, lock <-> manifest comparison and the golden schema hash.

use std::path::PathBuf;

use sortie_manifest::{
    Issue, Manifest, ScenarioLock, canonical_json, compare, ensure_valid, load_manifest, pb,
    schema_sha256, validate,
};
use sortie_proto::v1;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/op_north")
}

fn fixture() -> Manifest {
    load_manifest(root().join("logic/fixtures/op_north.manifest.pb")).unwrap()
}

fn lock() -> ScenarioLock {
    ScenarioLock::from_path(root().join("scenario.lock.json")).unwrap()
}

/// Paths of every issue for `m`, sorted.
fn paths(m: &Manifest) -> Vec<String> {
    let mut p: Vec<String> = validate(m).into_iter().map(|i| i.path).collect();
    p.sort();
    p
}

fn messages(m: &Manifest) -> String {
    validate(m)
        .iter()
        .map(Issue::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn golden_schema_hash_of_the_committed_lock() {
    let text = std::fs::read_to_string(root().join("scenario.lock.json")).unwrap();
    let lock: ScenarioLock = serde_json::from_str(&text).unwrap();
    // The canonical form, as produced by CPython's json.dumps(sort_keys=True,
    // ensure_ascii=False, separators=(",", ":")) over {"scenario", "schema", "symbols"}.
    let canonical = canonical_json(&lock.scenario, lock.schema, &lock.symbols).unwrap();
    assert_eq!(
        std::str::from_utf8(&canonical).unwrap(),
        r#"{"scenario":"op_north","schema":3,"symbols":{"airbases":[{"ident":"MAYKOP_KHANSKAYA"}],"flags":[{"ident":"SAM_NORTH_DOWN"}],"groups":[{"ident":"SAM_NORTH","kind":"ground","roles":["north","sam"]}],"menus":[{"ident":"SMOKE_OBJECTIVE"}],"templates":[{"ident":"RED_CAP","kind":"air","roles":[]}],"zones":[{"ident":"OBJECTIVE"},{"ident":"SAM_RING"}]}}"#
    );
    assert_eq!(
        schema_sha256(&lock.scenario, lock.schema, &lock.symbols).unwrap(),
        lock.schema_sha256
    );
    assert_eq!(
        lock.schema_sha256,
        "066934cb939700310641d6371b1f6140cfc86c2c3e04757d5a10f8a4664f270b"
    );
}

#[test]
fn hash_ignores_geometry_and_input_order() {
    let mut a = lock();
    let mut b = a.clone();
    b.symbols.zones.reverse();
    b.symbols.groups[0].roles.reverse();
    b.geometry.zones.clear();
    b.generator = "someone else".into();
    assert_eq!(
        schema_sha256(&a.scenario, a.schema, &a.symbols).unwrap(),
        schema_sha256(&b.scenario, b.schema, &b.symbols).unwrap()
    );
    a.symbols.groups[0].roles.push("east".into());
    assert_ne!(
        schema_sha256(&a.scenario, a.schema, &a.symbols).unwrap(),
        lock().schema_sha256
    );
}

#[test]
fn the_fixture_is_valid_and_matches_its_lock() {
    let m = fixture();
    ensure_valid(&m).unwrap();
    let d = compare(&lock(), &m);
    assert!(d.is_empty(), "{d}");
    assert!(d.is_compatible());
    assert_eq!(
        lock().symbol_keys(),
        vec![
            "airbase:MAYKOP_KHANSKAYA",
            "flag:SAM_NORTH_DOWN",
            "group:SAM_NORTH",
            "menu:SMOKE_OBJECTIVE",
            "template:RED_CAP",
            "zone:OBJECTIVE",
            "zone:SAM_RING",
        ]
    );
}

#[test]
fn lock_comparison_names_what_is_missing() {
    let mut m = fixture();
    m.zones.retain(|z| z.ident != "SAM_RING");
    m.flags.push(pb::Flag {
        ident: "NEW_FLAG".into(),
        name: "NEW_FLAG".into(),
        initial: 0,
    });
    m.templates[0].category = v1::Category::Helicopter as i32;
    m.groups[0].roles = vec!["sam".into()];
    let d = compare(&lock(), &m);
    assert_eq!(d.missing, vec!["zone:SAM_RING"]);
    assert_eq!(d.extra, vec!["flag:NEW_FLAG"]);
    assert_eq!(
        d.kind_changed,
        vec![(
            "template:RED_CAP".to_owned(),
            "air".to_owned(),
            "helo".to_owned()
        )]
    );
    assert_eq!(d.roles_changed, vec!["group:SAM_NORTH"]);
    assert!(d.hash.is_none(), "the manifest still claims the old hash");
    assert!(!d.is_compatible());
    let text = d.to_string();
    assert!(
        text.contains("missing from the manifest: zone:SAM_RING"),
        "{text}"
    );
    assert!(
        text.contains("template:RED_CAP is air in the lock but helo"),
        "{text}"
    );

    let mut other = fixture();
    other.scenario = "op_south".into();
    assert!(!compare(&lock(), &other).is_compatible());
}

#[test]
fn duplicate_names_and_symbols() {
    let mut m = fixture();
    let mut g = m.groups[0].clone();
    g.group_id = 50;
    for (k, u) in g.units.iter_mut().enumerate() {
        u.unit_id = 100 + k as u32;
    }
    m.groups.push(g);
    m.max_ids = Some(pb::MaxIds {
        group_id: 50,
        unit_id: 200,
    });
    let msgs = messages(&m);
    assert!(msgs.contains("groups[1].name: duplicate group name \"SAM North\" (also at groups[0].name) (coalition.addGroup would destroy the other object)"), "{msgs}");
    assert!(
        msgs.contains("groups[1].units[0].name: duplicate unit name \"SAM North-1\""),
        "{msgs}"
    );
    assert!(
        msgs.contains("groups[1].ident: duplicate group symbol \"SAM_NORTH\""),
        "{msgs}"
    );

    let mut m = fixture();
    m.player_flights[0].name = "SAM North".into();
    assert!(paths(&m).contains(&"player_flights[0].name".to_owned()));

    let mut m = fixture();
    m.zones[1].name = "Objective".into();
    m.flags.push(m.flags[0].clone());
    let p = paths(&m);
    assert!(p.contains(&"zones[1].name".to_owned()), "{p:?}");
    assert!(p.contains(&"flags[1].ident".to_owned()), "{p:?}");
    assert!(p.contains(&"flags[1].name".to_owned()), "{p:?}");
}

#[test]
fn id_rules() {
    // Placed units need their .miz ids, unique, covered by max_ids.
    let mut m = fixture();
    m.groups[0].units[1].unit_id = 1;
    m.groups[0].units[2].unit_id = 0;
    m.player_flights[0].group_id = 1;
    m.max_ids = Some(pb::MaxIds {
        group_id: 1,
        unit_id: 5,
    });
    let msgs = messages(&m);
    assert!(
        msgs.contains(
            "groups[0].units[1].unit_id: duplicate unitId 1 (also at groups[0].units[0].unit_id)"
        ),
        "{msgs}"
    );
    assert!(
        msgs.contains("groups[0].units[2].unit_id: placed units need the unitId"),
        "{msgs}"
    );
    assert!(
        msgs.contains("player_flights[0].group_id: duplicate groupId 1"),
        "{msgs}"
    );
    assert!(
        msgs.contains("max_ids.unit_id: is 5 but unitId 9 is used"),
        "{msgs}"
    );

    // Templates never carry ids (DCS assigns them at spawn).
    let mut m = fixture();
    m.templates[0].units[0].unit_id = 77;
    assert_eq!(paths(&m), vec!["templates[0].units[0].unit_id"]);

    let mut m = fixture();
    m.max_ids = None;
    assert_eq!(paths(&m), vec!["max_ids"]);
}

#[test]
fn geometry_rules() {
    let poly = |points: &[(f64, f64)]| {
        Some(pb::zone::Shape::Polygon(pb::Polygon {
            points: points
                .iter()
                .map(|&(north, east)| v1::MapPoint { north, east })
                .collect(),
        }))
    };
    let mut m = fixture();
    m.zones[0].shape = poly(&[(0.0, 0.0), (10.0, 10.0), (0.0, 10.0), (10.0, 0.0)]);
    m.zones[1].shape = Some(pb::zone::Shape::Circle(pb::Circle {
        center: Some(v1::MapPoint {
            north: f64::NAN,
            east: 0.0,
        }),
        radius_m: -1.0,
    }));
    m.groups[0].units[0].position = None;
    m.groups[0].units[1].alt_m = f64::INFINITY;
    m.groups[0].units[2].position = Some(v1::MapPoint {
        north: 9.0e6,
        east: 0.0,
    });
    m.groups[0].units[3].latlon = Some(v1::LatLon {
        lat: 91.0,
        lon: 0.0,
    });
    let msgs = messages(&m);
    for want in [
        "zones[0].polygon.points: edges 0 and 2 cross (the polygon intersects itself)",
        "zones[1].circle.center: coordinates must be finite",
        "zones[1].circle.radius_m: must be finite and > 0",
        "groups[0].units[0].position: position is missing",
        "groups[0].units[1].alt_m: must be finite",
        "groups[0].units[2].position: (9000000, 0) is outside the theatre plane",
        "groups[0].units[3].latlon: latitude/longitude out of range",
    ] {
        assert!(msgs.contains(want), "missing {want:?} in\n{msgs}");
    }

    let mut m = fixture();
    m.zones[0].shape = poly(&[(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    m.zones[1].shape = poly(&[(0.0, 0.0), (1.0, 1.0)]);
    let msgs = messages(&m);
    assert!(
        msgs.contains("zones[0].polygon.points: the polygon has zero area"),
        "{msgs}"
    );
    assert!(
        msgs.contains("zones[1].polygon.points: a polygon needs at least 3 points"),
        "{msgs}"
    );

    // A concave (but simple) polygon is fine.
    let mut m = fixture();
    m.zones[0].shape = poly(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 5.0),
        (5.0, 5.0),
        (5.0, 10.0),
        (0.0, 10.0),
    ]);
    assert_eq!(paths(&m), Vec::<String>::new());
}

#[test]
fn template_rules() {
    let mut m = fixture();
    let t = &mut m.templates[0];
    t.name = "Red CAP#1".into();
    t.country.clear();
    t.coalition = 0;
    t.dcs_group_json = r#"{"units": [{"name": "x"}]}"#.into();
    let msgs = messages(&m);
    for want in [
        "templates[0].name: must be non-empty and must not contain '#'",
        "templates[0].country: must not be empty",
        "templates[0].coalition: coalition must be RED, BLUE or NEUTRAL",
        "templates[0].dcs_group_json: has 1 units but the template lists 2",
    ] {
        assert!(msgs.contains(want), "missing {want:?} in\n{msgs}");
    }

    let mut m = fixture();
    m.templates[0].dcs_group_json = "[1, 2]".into();
    assert!(messages(&m).contains("must be a JSON object"));
    m.templates[0].dcs_group_json = "{".into();
    assert!(messages(&m).contains("is not valid JSON"));
    // pydcs-style Lua tables with string keys count too.
    m.templates[0].dcs_group_json = r#"{"units": {"1": {}, "2": {}}}"#.into();
    assert_eq!(paths(&m), Vec::<String>::new());

    let mut m = fixture();
    m.templates[0].units.clear();
    m.templates[0].category = v1::Category::Static as i32;
    let p = paths(&m);
    assert!(p.contains(&"templates[0].units".to_owned()), "{p:?}");
    assert!(p.contains(&"templates[0].category".to_owned()), "{p:?}");
}

#[test]
fn header_rules_and_error_display() {
    let mut m = fixture();
    m.scenario = "Op North".into();
    m.schema = 0;
    m.build_id = "not-a-ulid".into();
    m.start_time = "2026-11-01 06:30".into();
    m.theatre.clear();
    m.groups[0].roles = vec!["sam".into(), "north".into()];
    let p = paths(&m);
    for want in [
        "build_id",
        "groups[0].roles",
        "scenario",
        "schema",
        "start_time",
        "theatre",
    ] {
        assert!(p.contains(&want.to_owned()), "{want} not in {p:?}");
    }
    let err = ensure_valid(&m).unwrap_err().to_string();
    assert!(err.starts_with("invalid manifest ("), "{err}");
    assert!(
        err.contains("\n  - build_id: \"not-a-ulid\" is not a ULID"),
        "{err}"
    );

    let mut m = fixture();
    m.schema_sha256 = "0".repeat(64);
    assert_eq!(paths(&m), vec!["schema_sha256"]);
}

#[test]
fn ulid_and_time_formats() {
    use sortie_manifest::validate::{is_rfc3339_utc, is_ulid};
    assert!(is_ulid("01K8Y2Q0PH0000000000000000"));
    assert!(is_ulid("7ZZZZZZZZZZZZZZZZZZZZZZZZZ"));
    assert!(!is_ulid("8ZZZZZZZZZZZZZZZZZZZZZZZZZ"), "overflows 128 bits");
    assert!(!is_ulid("01K8Y2Q0PH000000000000000I"));
    assert!(!is_ulid("01k8y2q0ph0000000000000000"));
    assert!(is_rfc3339_utc("2026-11-01T06:30:00Z"));
    assert!(is_rfc3339_utc("2026-11-01T06:30:00.250Z"));
    assert!(!is_rfc3339_utc("2026-11-01T06:30:00+03:00"));
    assert!(!is_rfc3339_utc("2026-13-01T06:30:00Z"));
    assert!(!is_rfc3339_utc("2026-11-01T06:30:00.Z"));
}
