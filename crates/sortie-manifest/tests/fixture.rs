//! The op_north fixtures round-trip with prost and agree with each other.

use std::path::PathBuf;

use pretty_assertions::assert_eq;
use sortie_manifest::{
    ScenarioLock, decode_manifest, digest, encode_manifest, load_manifest, validate,
};

fn op_north_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/op_north")
}

#[test]
fn manifest_fixture_round_trips() {
    let path = op_north_dir().join("logic/fixtures/op_north.manifest.pb");
    let bytes = std::fs::read(&path).expect("fixture exists (cargo xtask fixtures)");
    let m = decode_manifest(&bytes).expect("decodes");
    assert_eq!(
        encode_manifest(&m),
        bytes,
        "prost re-encoding is byte-identical"
    );
    assert_eq!(load_manifest(&path).unwrap(), m);

    assert_eq!(m.scenario, "op_north");
    assert_eq!(m.time_limit_s, 3 * 3600);
    let sam = sortie_manifest::group(&m, "SAM_NORTH").expect("SAM_NORTH");
    assert_eq!(sam.name, "SAM North");
    assert_eq!(sam.units.len(), 7);
    assert_eq!(
        sam.units
            .iter()
            .filter(|u| u.type_name == "S-300PS 5P85C ln")
            .count(),
        4
    );
    assert_eq!(
        sortie_manifest::template(&m, "RED_CAP")
            .unwrap()
            .units
            .len(),
        2
    );
    assert_eq!(
        sortie_manifest::airbase(&m, "MAYKOP_KHANSKAYA")
            .unwrap()
            .name,
        "Maykop-Khanskaya"
    );
    assert!(sortie_manifest::flag(&m, "SAM_NORTH_DOWN").is_some());
    assert_eq!(
        sortie_manifest::menu(&m, "SMOKE_OBJECTIVE").unwrap().label,
        "Smoke the objective"
    );
    assert_eq!(m.player_flights[0].name, "Blue 1");

    let issues = validate(&m);
    assert!(issues.is_empty(), "fixture is valid: {issues:?}");
    assert_eq!(digest(&bytes).len(), 64);
}

#[test]
fn lock_fixture_matches_manifest() {
    let lock =
        ScenarioLock::from_path(op_north_dir().join("scenario.lock.json")).expect("lock verifies");
    let m = load_manifest(op_north_dir().join("logic/fixtures/op_north.manifest.pb")).unwrap();
    assert_eq!(lock.schema_sha256, m.schema_sha256);
    let derived = ScenarioLock::from_manifest(&m, lock.generator.clone()).unwrap();
    assert_eq!(derived, lock);
    // The file layout is what Python's json.dumps(indent=2, sort_keys=True) + "\n" writes.
    let text = std::fs::read_to_string(op_north_dir().join("scenario.lock.json")).unwrap();
    assert_eq!(lock.to_json_pretty().unwrap(), text);
}

#[test]
fn tampered_lock_is_rejected() {
    let text = std::fs::read_to_string(op_north_dir().join("scenario.lock.json")).unwrap();
    let tampered = text.replace("\"SAM_RING\"\n", "\"SAM_RINGS\"\n");
    assert_ne!(tampered, text);
    let err = ScenarioLock::from_json_str(&tampered).unwrap_err();
    assert!(err.to_string().contains("schema_sha256 mismatch"), "{err}");
}
