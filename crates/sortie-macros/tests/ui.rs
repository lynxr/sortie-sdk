//! Compile errors of the macros, pinned as `.stderr` files. Regenerate after an intended
//! change with `TRYBUILD=overwrite cargo test -p sortie-macros --test ui` and review the diff.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
