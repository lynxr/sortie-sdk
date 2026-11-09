//! # Sortie SDK
//!
//! Mission logic for DCS World as an ordinary async Rust program. Reading the world is
//! synchronous and local (a replica fed by the engine); writing returns futures that complete
//! after DCS actually executed the command.
//!
//! ```ignore
//! use sortie::prelude::*;
//! sortie::include_scenario!(op_north);
//!
//! #[sortie::mission]
//! pub async fn run(ctx: Ctx<op_north::OpNorth>, start: Start) -> sortie::Result<()> { ... }
//! ```
//!
//! * [`Ctx<M>`] is the session context typed by the scenario marker `M` (cheap to clone, not
//!   `Send`: the runtime is single-threaded);
//! * every write is a [`Cmd`] builder with `.key()`, `.timeout()`, `.after()`, awaited for its
//!   result; failures are [`Error`]s that name the Rust call site;
//! * waiting is by model time and world state: `ctx.sleep`, `ctx.timer`, `ctx.wait_until`,
//!   `ctx.events()`;
//! * [`runtime`] runs a mission (one task per session, `current_thread` + `LocalSet`),
//!   [`connect`] opens `Logic.Attach` over gRPC, [`main!`] is the whole `main.rs`;
//! * [`testing::TestHarness`] is what `#[sortie::test]` drives (implemented by FakeDcs, which
//!   is not part of SDK 0.1: there `#[sortie::test]` is a compile error that says so).

pub mod cmd;
mod ctx;
mod error;
pub mod events;
pub mod group;
pub mod menu;
#[cfg(doctest)]
mod mission_checks;
pub mod prelude;
pub mod runtime;
pub mod state;
pub mod task;
pub mod testing;

pub use cmd::Cmd;
pub use ctx::{Ctx, FlagHandle, FlagValue, MessageTarget, Position, Report, Start, TaskHandle};
pub use error::{Error, Failure, Result};
pub use events::{EventType, Events, TypedEvents};
pub use group::{AiOption, AnyGroup, Group};
pub use menu::{
    CoalitionPress, CoalitionWide, GroupPress, Menu, MenuBuilder, MenuCommand, PerGroup, Presses,
};
pub use runtime::{EndReason, LogicHandle, SessionEnd, connect};
/// Re-exported for `#[derive(Serialize, Deserialize)]` users (the user crate still needs a
/// direct `serde` dependency for the derive macros).
pub use serde;
/// `sortie::include_scenario!(op_north);` defines `pub mod op_north` from the code that
/// `sortie_build::scenario(name, lock).emit()` generated in `build.rs`: the marker `OpNorth`
/// and the `groups`, `zones`, `templates`, `flags`, `airbases`, `menus` modules. A missing
/// build script or a wrong name is reported at the call.
pub use sortie_macros::include_scenario;
pub use sortie_macros::{mission, test};
/// Wire types (`sortie.v1`, `sortie.manifest.v1`, ...).
pub use sortie_proto as proto;
/// Shared vocabulary; generated scenario code refers to `::sortie::types::*`.
pub use sortie_types as types;
/// The world model behind `ctx.wait_until(|w| ...)`.
pub use sortie_world as world;
pub use sortie_world::World;
pub use state::StateDoc;
pub use task::Task;

/// Runs a logic binary: what [`main!`] expands to. Parses `--engine` / `SORTIE_ENGINE`,
/// `SORTIE_TOKEN` and the other [`runtime::Options`], attaches to the engine and serves
/// sessions until the engine shuts the logic down, reconnecting with backoff (1 s … 30 s)
/// when the connection drops.
pub fn run<M: types::Scenario, F: runtime::Mission<M>>(
    mission: F,
    logic: &str,
    version: &str,
) -> std::process::ExitCode {
    runtime::main_named(mission, logic, version)
}

/// The whole `main.rs` of a logic binary: `sortie::main!(my_logic::run);`.
///
/// Expands to `fn main() -> ExitCode` calling [`run`] with the binary's name and version.
#[macro_export]
macro_rules! main {
    ($mission:path) => {
        fn main() -> ::std::process::ExitCode {
            $crate::run(
                $mission,
                ::core::env!("CARGO_BIN_NAME"),
                ::core::env!("CARGO_PKG_VERSION"),
            )
        }
    };
}
