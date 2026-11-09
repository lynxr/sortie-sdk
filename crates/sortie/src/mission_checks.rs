//! Compile-fail doctests of `#[sortie::mission]`, built only by `cargo test --doc`.
//!
//! The macro checks the syntax itself (its messages are pinned by the trybuild tests in
//! `crates/sortie-macros/tests/ui`) and leaves the rest to hidden compiler type checks. Their
//! messages are rustc's and change between toolchains, so they are pinned here by error code
//! only. The first example compiles (and shows that aliases pass the syntax checks); each of
//! the others changes one thing in it.
//!
//! ```
//! use sortie::prelude::*;
//! use sortie::types::{Scenario, Symbol};
//!
//! #[derive(Clone, Copy, Debug, Default)]
//! pub struct Demo;
//!
//! impl Scenario for Demo {
//!     const NAME: &'static str = "demo";
//!     const SCHEMA: u32 = 1;
//!     const SCHEMA_SHA256: &'static str = "";
//!     const SYMBOLS: &'static [Symbol] = &[];
//! }
//!
//! type DemoCtx = Ctx<Demo>;
//! type MissionResult = sortie::Result<()>;
//!
//! #[sortie::mission]
//! pub async fn run(_ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
//!     Ok(())
//! }
//!
//! #[sortie::mission]
//! pub async fn aliased(_ctx: DemoCtx, _start: sortie::Start) -> MissionResult {
//!     Ok(())
//! }
//!
//! fn main() {}
//! ```
//!
//! A context whose marker is not a scenario (the bound `M: Scenario` of the type checks):
//!
//! ```compile_fail,E0277
//! use sortie::prelude::*;
//!
//! #[derive(Clone, Copy, Debug, Default)]
//! pub struct NotAScenario;
//!
//! #[sortie::mission]
//! pub async fn run(_ctx: Ctx<NotAScenario>, _start: Start) -> sortie::Result<()> {
//!     Ok(())
//! }
//!
//! fn main() {}
//! ```
//!
//! An alias of a wrong return type passes the syntax checks and fails the type checks:
//!
//! ```compile_fail,E0308
//! # use sortie::prelude::*;
//! # use sortie::types::{Scenario, Symbol};
//! # #[derive(Clone, Copy, Debug, Default)]
//! # pub struct Demo;
//! # impl Scenario for Demo {
//! #     const NAME: &'static str = "demo";
//! #     const SCHEMA: u32 = 1;
//! #     const SCHEMA_SHA256: &'static str = "";
//! #     const SYMBOLS: &'static [Symbol] = &[];
//! # }
//! type MissionResult = Result<(), String>;
//!
//! #[sortie::mission]
//! pub async fn run(_ctx: Ctx<Demo>, _start: Start) -> MissionResult {
//!     Ok(())
//! }
//! # fn main() {}
//! ```
//!
//! Swapped arguments are a syntax error of the macro (a `compile_error!`, which has no error
//! code):
//!
//! ```compile_fail
//! # use sortie::prelude::*;
//! # use sortie::types::{Scenario, Symbol};
//! # #[derive(Clone, Copy, Debug, Default)]
//! # pub struct Demo;
//! # impl Scenario for Demo {
//! #     const NAME: &'static str = "demo";
//! #     const SCHEMA: u32 = 1;
//! #     const SCHEMA_SHA256: &'static str = "";
//! #     const SYMBOLS: &'static [Symbol] = &[];
//! # }
//! #[sortie::mission]
//! pub async fn run(_start: Start, _ctx: Ctx<Demo>) -> sortie::Result<()> {
//!     Ok(())
//! }
//! # fn main() {}
//! ```
