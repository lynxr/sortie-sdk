//! Readable errors for bad locks, and codegen for shapes the op_north lock does not have.

use std::path::Path;

use sortie_build::BuildError;
use sortie_manifest::{
    IdentSymbol, KindSymbol, LOCK_FORMAT, ScenarioLock, SymbolSet, ZoneGeometry,
};
use sortie_types::MapPoint;

const OP_NORTH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/op_north/scenario.lock.json"
);

fn write(dir: &Path, name: &str, text: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, text).unwrap();
    p
}

fn lock(scenario: &str, symbols: SymbolSet, zones: Vec<(&str, ZoneGeometry)>) -> ScenarioLock {
    let schema_sha256 = sortie_manifest::schema_sha256(scenario, 1, &symbols).unwrap();
    ScenarioLock {
        format: LOCK_FORMAT,
        generator: "test".into(),
        geometry: sortie_manifest::GeometryLock {
            zones: zones.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
        },
        scenario: scenario.into(),
        schema: 1,
        schema_sha256,
        symbols,
    }
}

fn err_text(e: &BuildError) -> String {
    format!("{e}\nhelp: {}", e.help())
}

#[test]
fn missing_lock_names_the_path_and_the_fix() {
    let e = sortie_build::scenario("op_north", "/nonexistent/scenario.lock.json")
        .generate()
        .unwrap_err();
    let t = err_text(&e);
    assert!(t.contains("/nonexistent/scenario.lock.json"), "{t}");
    assert!(t.contains("sortie-gen lock"), "{t}");
}

#[test]
fn broken_json_says_where() {
    let dir = tempfile::tempdir().unwrap();
    let text =
        std::fs::read_to_string(OP_NORTH)
            .unwrap()
            .replacen("\"format\": 1,", "\"format\": 1", 1);
    let p = write(dir.path(), "scenario.lock.json", &text);
    let e = sortie_build::scenario("op_north", &p)
        .generate()
        .unwrap_err();
    let t = err_text(&e);
    assert!(
        t.contains("scenario.lock.json: invalid scenario.lock.json"),
        "{t}"
    );
    assert!(t.contains("line 3"), "{t}");
}

#[test]
fn edited_lock_is_refused_with_a_hint() {
    let dir = tempfile::tempdir().unwrap();
    let text = std::fs::read_to_string(OP_NORTH)
        .unwrap()
        .replace("SAM_NORTH_DOWN", "SAM_DOWN");
    let p = write(dir.path(), "scenario.lock.json", &text);
    let e = sortie_build::scenario("op_north", &p)
        .generate()
        .unwrap_err();
    let t = err_text(&e);
    assert!(t.contains("schema_sha256 mismatch"), "{t}");
    assert!(t.contains("instead of editing it"), "{t}");
}

#[test]
fn keyword_scenarios_and_bad_geometry_are_refused() {
    let k = lock("type", SymbolSet::default(), vec![]);
    let e = sortie_build::generate(&k, "::sortie").unwrap_err();
    assert!(matches!(e, BuildError::Keyword(_)), "{e}");
    assert!(sortie_build::is_keyword("gen"), "reserved in edition 2024");

    let zones = SymbolSet {
        zones: vec![IdentSymbol { ident: "Z".into() }],
        ..SymbolSet::default()
    };
    let missing = lock("s", zones.clone(), vec![]);
    assert!(matches!(
        sortie_build::generate(&missing, "::sortie"),
        Err(BuildError::MissingGeometry(_))
    ));
    let bad = lock(
        "s",
        zones,
        vec![(
            "Z",
            ZoneGeometry::Circle {
                center: MapPoint::new(0.0, 0.0),
                radius_m: 0.0,
            },
        )],
    );
    let e = sortie_build::generate(&bad, "::sortie").unwrap_err();
    assert!(e.to_string().contains("radius > 0"), "{e}");
}

#[test]
fn write_to_only_rewrites_on_change() {
    let dir = tempfile::tempdir().unwrap();
    let b = sortie_build::scenario("op_north", OP_NORTH);
    let out = b.write_to(dir.path()).unwrap();
    assert_eq!(out.file_name().unwrap(), "sortie_scenario_op_north.rs");
    let before = std::fs::metadata(&out).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    b.write_to(dir.path()).unwrap();
    assert_eq!(std::fs::metadata(&out).unwrap().modified().unwrap(), before);
}

#[test]
fn every_kind_and_polygon_zones() {
    let k = |ident: &str, kind: &str, roles: &[&str]| KindSymbol {
        ident: ident.into(),
        kind: kind.into(),
        roles: roles.iter().map(|r| (*r).to_owned()).collect(),
    };
    let symbols = SymbolSet {
        groups: vec![
            k("CONVOY", "ground", &[]),
            k("FLEET", "ship", &["navy"]),
            k("RESCUE", "helo", &[]),
        ],
        templates: vec![k("STRIKE", "air", &["blue-strike"])],
        zones: vec![IdentSymbol {
            ident: "BOX".into(),
        }],
        ..SymbolSet::default()
    };
    let l = lock(
        "caucasus_2",
        symbols,
        vec![(
            "BOX",
            ZoneGeometry::Polygon {
                points: vec![
                    MapPoint::new(-1.5, 0.0),
                    MapPoint::new(0.0, 2.0),
                    MapPoint::new(1.0, -3.25),
                ],
            },
        )],
    );
    l.verify().unwrap();
    let src = sortie_build::generate_with_types_path(&l, "::sortie_types").unwrap();
    assert!(src.contains("pub struct Caucasus2;"));
    assert!(src.contains("::sortie_types::GroupRef<super::Caucasus2, ::sortie_types::Ship>"));
    assert_eq!(
        sortie_build::generate(&l, "crate").unwrap(),
        sortie_build::generate_with_types_path(&l, "crate::types").unwrap()
    );
    // The checked-in copy is compiled below (`generated` module), so this also proves that
    // the generated code type-checks against sortie-types.
    let golden = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/caucasus_2.rs");
    if std::env::var_os("SORTIE_BLESS").is_some() {
        std::fs::write(golden, &src).unwrap();
    }
    assert_eq!(
        src,
        std::fs::read_to_string(golden).unwrap(),
        "rerun with SORTIE_BLESS=1 to update tests/data/caucasus_2.rs"
    );
}

#[allow(dead_code, unreachable_pub)]
mod generated {
    include!("data/caucasus_2.rs");
}

#[test]
fn generated_handles_work() {
    use generated::{Caucasus2, groups, templates, zones};
    use sortie_types::{Kind, Scenario, Ship};
    assert_eq!(Caucasus2::NAME, "caucasus_2");
    assert_eq!(Caucasus2::SYMBOLS.len(), 5);
    assert_eq!(groups::FLEET.kind_name(), Ship::NAME);
    assert_eq!(templates::STRIKE.ident(), "STRIKE");
    assert!(zones::BOX.contains(MapPoint::new(0.0, 0.5)));
    assert!(!zones::BOX.contains(MapPoint::new(5.0, 5.0)));
    assert_eq!(
        sortie_types::schema_sha256_bytes::<Caucasus2>()
            .unwrap()
            .len(),
        32
    );
}
