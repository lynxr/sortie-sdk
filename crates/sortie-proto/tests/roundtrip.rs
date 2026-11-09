//! Encoding round trips for the generated messages and the hand-written helpers.

use proptest::prelude::*;
use prost::Message;
#[cfg(feature = "link")]
use sortie_proto::link::v1 as link;
use sortie_proto::manifest::v1 as manifest;
use sortie_proto::v1;
use sortie_proto::{LuaTable, LuaValue, eid};

fn round_trip<M: Message + Default + PartialEq + std::fmt::Debug>(m: &M) {
    let bytes = m.encode_to_vec();
    let back = M::decode(bytes.as_slice()).expect("decode");
    assert_eq!(&back, m);
}

#[cfg(feature = "link")]
fn sample_raw_event() -> LuaValue {
    let initiator = LuaTable::new()
        .with("id_", 16_777_472)
        .with("name", "SAM North-1")
        .with("type", "S-300PS 54K6 cp")
        .with("coalition", 1)
        .with("category", 2)
        .with(
            "group",
            LuaTable::new().with("id", 1).with("name", "SAM North"),
        );
    LuaValue::table(
        LuaTable::new()
            .with("id", 8)
            .with("time", 1234.5)
            .with("initiator", initiator),
    )
}

#[cfg(feature = "link")]
#[test]
fn link_frames_round_trip() {
    let frames = vec![
        link::Frame {
            seq: 1,
            ack: 0,
            body: Some(link::frame::Body::Hello(link::Hello {
                env: v1::Env::Mission as i32,
                bridge_version: "0.1.0".into(),
                link_major: 1,
                link_minor: 0,
                dcs_version: "2.9.31.29434".into(),
                layer_hashes: [("layer/events.lua".to_string(), "ab".repeat(32))]
                    .into_iter()
                    .collect(),
                client_nonce: vec![7; 32],
                writedir: r"D:\Saved Games\DCS.server\".into(),
                pid: 4242,
                initial_credits: 64,
                capabilities: vec!["samples".into()],
            })),
        },
        link::Frame {
            seq: 9,
            ack: 4,
            body: Some(link::frame::Body::RawEvents(link::RawEvents {
                events: vec![sample_raw_event()],
            })),
        },
        link::Frame {
            seq: 10,
            ack: 4,
            body: Some(link::frame::Body::Exec(link::ExecCommand {
                id: "01JC70ZZZZZZZZZZZZZZZZZZZZ".into(),
                op: "set_option".into(),
                args: Some(LuaValue::table(
                    LuaTable::new()
                        .with("group", "SAM North")
                        .with("option", 9)
                        .with("value", 2),
                )),
                deadline_ms: 5000,
                env: v1::Env::Mission as i32,
            })),
        },
        link::Frame {
            seq: 11,
            ack: 10,
            body: Some(link::frame::Body::ExecResult(link::ExecResult {
                id: "01JC70ZZZZZZZZZZZZZZZZZZZZ".into(),
                outcome: Some(link::exec_result::Outcome::Err(link::ExecError {
                    code: link::ExecErrorCode::Gone as i32,
                    message: "Unit doesn't exist".into(),
                    traceback: "stack traceback:\n\t[C]: in function 'getController'".into(),
                })),
                cost_us: 37,
            })),
        },
        link::Frame {
            seq: 12,
            ack: 10,
            body: Some(link::frame::Body::Tables(link::Tables {
                tables: [(
                    "world.event".to_string(),
                    link::NumberTable {
                        entries: [
                            ("S_EVENT_SHOT".to_string(), 1),
                            ("S_EVENT_DEAD".to_string(), 8),
                        ]
                        .into_iter()
                        .collect(),
                    },
                )]
                .into_iter()
                .collect(),
            })),
        },
    ];
    for f in &frames {
        round_trip(f);
    }
}

#[test]
fn command_and_event_round_trip() {
    let cmd = v1::Command {
        id: "01JC70AAAAAAAAAAAAAAAAAAAA".into(),
        key: "red-cap-1".into(),
        lane: v1::Lane::Bulk as i32,
        timeout_ms: 5000,
        on_pause: v1::PausePolicy::Wait as i32,
        after: String::new(),
        origin: Some(v1::Origin {
            logic: "op_north_logic".into(),
            file: "src/lib.rs".into(),
            line: 48,
            ..Default::default()
        }),
        session: String::new(),
        body: Some(v1::command::Body::SpawnGroup(v1::SpawnGroup {
            template: "RED_CAP".into(),
            name: Some("Red CAP 1".into()),
            placement: Some(v1::spawn_group::Placement::FromAirbase(v1::FromAirbase {
                airbase: "MAYKOP_KHANSKAYA".into(),
                start: v1::StartType::Runway as i32,
            })),
        })),
    };
    round_trip(&cmd);

    let end = v1::Command {
        id: "01JC70BBBBBBBBBBBBBBBBBBBB".into(),
        body: Some(v1::command::Body::EndSession(v1::EndSession {
            outcome: v1::Outcome::BlueWin as i32,
            text: "CAP down".into(),
        })),
        ..Default::default()
    };
    round_trip(&end);
    // EndSession must be field 25 of the Command body: tag (25 << 3) | 2 = 202 = varint CA 01.
    let bytes = end.encode_to_vec();
    assert!(bytes.windows(2).any(|w| w == [0xCA, 0x01]));

    let ev = v1::Event {
        seq: 42,
        model_time: 812.25,
        wall_time: Some(prost_types::Timestamp {
            seconds: 1_793_000_000,
            nanos: 0,
        }),
        session: "01JC6Z0000000000000000ABCD".into(),
        kind: Some(v1::event::Kind::Dead(v1::Dead {
            initiator: Some(v1::EntitySnap {
                eid: eid::pack(3, 1),
                name: "SAM North-1".into(),
                type_name: "S-300PS 54K6 cp".into(),
                coalition: v1::Coalition::Red as i32,
                category: v1::Category::GroundUnit as i32,
                pos: Some(v1::Vec3 {
                    x: -32798.0,
                    y: 180.0,
                    z: 463776.3,
                }),
                dead: true,
                ..Default::default()
            }),
        })),
    };
    round_trip(&ev);
}

#[test]
fn manifest_round_trip() {
    let m = manifest::Manifest {
        scenario: "op_north".into(),
        schema: 3,
        schema_sha256: "00".repeat(32),
        zones: vec![manifest::Zone {
            ident: "SAM_RING".into(),
            name: "SAM Ring".into(),
            shape: Some(manifest::zone::Shape::Circle(manifest::Circle {
                center: Some(v1::MapPoint {
                    north: -32798.0,
                    east: 463776.3,
                }),
                radius_m: 60_000.0,
            })),
        }],
        ..Default::default()
    };
    round_trip(&m);
}

#[test]
fn file_descriptor_set_lists_all_packages() {
    let fds = prost_types::FileDescriptorSet::decode(sortie_proto::FILE_DESCRIPTOR_SET).unwrap();
    let packages: std::collections::BTreeSet<_> =
        fds.file.iter().filter_map(|f| f.package.clone()).collect();
    for p in ["sortie.v1", "sortie.manifest.v1"] {
        assert!(packages.contains(p), "missing {p}");
    }
    // The private link protocol is compiled only for the engine side.
    assert_eq!(packages.contains("sortie.link.v1"), cfg!(feature = "link"));
}

fn arb_lua() -> impl Strategy<Value = LuaValue> {
    let leaf = prop_oneof![
        Just(LuaValue::nil()),
        any::<bool>().prop_map(LuaValue::boolean),
        (-1.0e9f64..1.0e9).prop_map(LuaValue::number),
        "[a-zA-Z0-9 _-]{0,12}".prop_map(LuaValue::string),
    ];
    leaf.prop_recursive(3, 32, 6, |inner| {
        prop::collection::vec(("[a-z_]{1,6}", inner), 0..6)
            .prop_map(|pairs| LuaValue::table(pairs.into_iter().collect::<LuaTable>()))
    })
}

proptest! {
    #[test]
    fn lua_value_prost_round_trip(v in arb_lua()) {
        let bytes = v.encode_to_vec();
        let back = LuaValue::decode(bytes.as_slice()).unwrap();
        prop_assert_eq!(back, v);
    }

    #[test]
    fn lua_value_json_bridge_is_stable(v in arb_lua()) {
        // JSON -> Lua -> JSON is the identity on the image of lua_to_json.
        let j = sortie_proto::lua::lua_to_json(&v);
        let again = sortie_proto::lua::lua_to_json(&sortie_proto::lua::json_to_lua(&j));
        prop_assert_eq!(again, j);
    }
}
