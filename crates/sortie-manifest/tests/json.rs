//! `manifest.json` round trips: the committed op_north JSON is exactly what Python protobuf
//! writes for the committed `.pb`, and an edge-case manifest (floats, escapes, unknown enum
//! values, maps) matches CPython 3.13 + protobuf 7 output byte for byte.

use std::path::{Path, PathBuf};

use pretty_assertions::assert_eq;
use sortie_manifest::{
    Manifest, decode_manifest, encode_manifest, load_manifest, manifest_from_json,
    manifest_to_json, pb,
};
use sortie_proto::v1;

fn op_north() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/op_north/logic/fixtures")
}

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data")
}

fn check_pair(pb_path: &Path, json_path: &Path) {
    let bytes = std::fs::read(pb_path).unwrap();
    let from_pb = decode_manifest(&bytes).unwrap();
    let json = std::fs::read_to_string(json_path).unwrap();
    assert_eq!(
        manifest_to_json(&from_pb),
        json,
        "Rust writes what Python wrote"
    );
    let from_json = manifest_from_json(&json).unwrap();
    // NaN != NaN, so compare re-encodings.
    assert_eq!(encode_manifest(&from_json), encode_manifest(&from_pb));
    assert_eq!(
        encode_manifest(&load_manifest(json_path).unwrap()),
        encode_manifest(&load_manifest(pb_path).unwrap())
    );
}

#[test]
fn op_north_json_is_the_python_rendering_of_the_pb() {
    check_pair(
        &op_north().join("op_north.manifest.pb"),
        &op_north().join("op_north.manifest.json"),
    );
}

#[test]
fn edge_cases_match_python() {
    check_pair(
        &data().join("edge.manifest.pb"),
        &data().join("edge.manifest.json"),
    );
}

#[test]
fn camel_case_json_from_other_tools_is_accepted() {
    let m = manifest_from_json(
        r#"{"scenario": "x", "schemaSha256": "ab", "playerFlights": [{"name": "Blue 1",
            "typeName": "F-16C_50", "start": "START_TYPE_COLD", "groupId": "7"}],
            "zones": [{"ident": "Z", "polygon": {"points": [{"north": 1, "east": "NaN"}]}}]}"#,
    )
    .unwrap();
    assert_eq!(m.schema_sha256, "ab");
    let f = &m.player_flights[0];
    assert_eq!(f.type_name, "F-16C_50");
    assert_eq!(f.start, v1::StartType::Cold as i32);
    assert_eq!(f.group_id, 7);
    let Some(pb::zone::Shape::Polygon(p)) = &m.zones[0].shape else {
        panic!("polygon expected");
    };
    assert!(p.points[0].east.is_nan());
}

/// The edge-case manifest. Regenerate the data files with
/// `cargo test -p sortie-manifest --test json -- --ignored` and then
/// `python -c "...json_format.MessageToJson(m, preserving_proto_field_name=True, indent=2)..."`
/// (see crates/sortie-manifest/tests/data/README.md).
fn edge() -> Manifest {
    let mp = |north: f64, east: f64| Some(v1::MapPoint { north, east });
    let floats = [
        0.1 + 0.2,
        1e-5,
        1.5e-7,
        1e-4,
        1e15,
        1e16,
        1e22,
        -0.0,
        5e-324,
        1.7976931348623157e308,
        123456.789,
        -32798.01604360901,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2.5,
        -1e-300,
        9007199254740993.0,
        0.000123456789,
        33.333333333333336,
    ];
    let units = floats
        .iter()
        .enumerate()
        .map(|(i, f)| pb::Unit {
            name: format!("U{i}"),
            type_name: "T".into(),
            // JSON has no negative NaN: keep NaN positive so the round trip is exact.
            position: mp(*f, if f.is_nan() { *f } else { -*f }),
            alt_m: *f,
            heading_rad: 0.0,
            unit_id: i as u32 + 1,
            latlon: Some(v1::LatLon { lat: *f, lon: 0.0 }),
            skill: String::new(),
        })
        .collect();
    Manifest {
        scenario: "edge".into(),
        schema: 1,
        schema_sha256: String::new(),
        build_id: "01K8Y2Q0PH0000000000000000".into(),
        theatre: "Caucasus".into(),
        start_time: String::new(),
        time_limit_s: 0,
        groups: vec![pb::Group {
            ident: "G".into(),
            name: "Майкоп \"Север\" \\ 1\n\t\u{1f6e9} é".into(),
            category: 77,
            coalition: v1::Coalition::Neutral as i32,
            country: "Russia".into(),
            country_id: -1,
            roles: vec!["a".into(), "b".into()],
            units,
            group_id: 4_000_000_000,
            late_activation: true,
        }],
        zones: vec![
            pb::Zone {
                ident: "P".into(),
                name: "Poly".into(),
                shape: Some(pb::zone::Shape::Polygon(pb::Polygon {
                    points: vec![
                        v1::MapPoint::default(),
                        v1::MapPoint {
                            north: 1.0,
                            east: 0.0,
                        },
                        v1::MapPoint {
                            north: 1.0,
                            east: 1.0,
                        },
                    ],
                })),
            },
            pb::Zone {
                ident: "E".into(),
                name: "Empty circle".into(),
                shape: Some(pb::zone::Shape::Circle(pb::Circle::default())),
            },
            pb::Zone {
                ident: "N".into(),
                name: "No shape".into(),
                shape: None,
            },
        ],
        templates: vec![pb::Template {
            ident: "T".into(),
            name: "Tmpl".into(),
            dcs_group_json: r#"{"units": [{"name": "x"}], "task": "CAP"}"#.into(),
            ..Default::default()
        }],
        flags: vec![pb::Flag {
            ident: "F".into(),
            name: "flag".into(),
            initial: -5,
        }],
        airbases: vec![pb::Airbase::default()],
        menus: vec![pb::Menu {
            ident: "M".into(),
            label: "Label".into(),
            path: vec!["Sortie".into(), "Sub".into()],
        }],
        player_flights: vec![],
        max_ids: Some(pb::MaxIds::default()),
        tools: Some(pb::ToolVersions {
            generator: "edge".into(),
            pydcs: String::new(),
            python: String::new(),
            extra: [("only".to_owned(), "one".to_owned())]
                .into_iter()
                .collect(),
        }),
    }
}

#[test]
#[ignore = "writes tests/data/edge.manifest.pb; run by hand when the edge case changes"]
fn write_edge_fixture() {
    std::fs::write(data().join("edge.manifest.pb"), encode_manifest(&edge())).unwrap();
}

#[test]
fn edge_fixture_is_current() {
    let bytes = std::fs::read(data().join("edge.manifest.pb")).unwrap();
    assert_eq!(
        bytes,
        encode_manifest(&edge()),
        "run write_edge_fixture and regenerate the JSON"
    );
}
