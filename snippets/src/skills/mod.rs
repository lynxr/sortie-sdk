//! Compiled examples of the agent skills (`sdk/agents`), checked against the markdown by
//! `cargo xtask docs-check`.
//!
//! The examples are written for the `frontline` scenario of
//! `sdk/snippets/python/skills_scenario.py`; [`frontline`] is the module that
//! `sortie::include_scenario!(frontline)` would include for its lock, so no build script is
//! needed here.

#[allow(dead_code, missing_docs, unreachable_pub, clippy::all)]
pub mod frontline;

pub mod errors;
pub mod mission_logic;
pub mod patterns;
