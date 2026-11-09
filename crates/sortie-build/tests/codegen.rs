//! Snapshot of the code generated for the op_north lock fixture.

#[test]
fn op_north_codegen() {
    let lock_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/op_north/scenario.lock.json"
    );
    let source = sortie_build::scenario("op_north", lock_path)
        .generate()
        .expect("generates");
    assert!(source.contains("pub struct OpNorth;"));
    assert!(source.contains("const SCHEMA: u32 = 3;"));
    assert!(source.contains("pub const SAM_NORTH: ::sortie::types::GroupRef<"));
    insta::assert_snapshot!(source);
}

#[test]
fn wrong_scenario_name_is_an_error() {
    let lock_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/op_north/scenario.lock.json"
    );
    let err = sortie_build::scenario("op_south", lock_path)
        .generate()
        .unwrap_err();
    assert!(err.to_string().contains("op_south"), "{err}");
}

#[test]
fn marker_names() {
    assert_eq!(sortie_build::marker_name("op_north"), "OpNorth");
    assert_eq!(sortie_build::marker_name("caucasus_2"), "Caucasus2");
}
