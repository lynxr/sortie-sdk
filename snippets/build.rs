//! The scenario symbols of the docs' examples (`crate::docs::my_mission`), from the lock that
//! `cargo xtask fixtures` writes for `python/docs_scenario.py`.

fn main() {
    sortie_build::scenario("my_mission", "scenario/scenario.lock.json").emit();
}

/// The build script of a client's logic crate, as the docs show it: compiled here, never run
/// (a client keeps `scenario.lock.json` next to `Cargo.toml`).
#[allow(dead_code)]
mod client {
    // snippet:start template-build-rs
    fn main() {
        sortie_build::scenario("my_mission", "scenario.lock.json").emit();
    }
    // snippet:end template-build-rs
}
