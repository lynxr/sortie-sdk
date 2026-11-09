// Types that are wrong as written. A marker that is not a scenario is only known to the
// compiler: see the `compile_fail` doctests in `crates/sortie/src/mission_checks.rs`.

use sortie::prelude::*;
use sortie::types::{Scenario, Symbol};

#[derive(Clone, Copy, Debug, Default)]
pub struct Demo;

impl Scenario for Demo {
    const NAME: &'static str = "demo";
    const SCHEMA: u32 = 1;
    const SCHEMA_SHA256: &'static str = "";
    const SYMBOLS: &'static [Symbol] = &[];
}

#[sortie::mission]
pub async fn swapped(_start: Start, _ctx: Ctx<Demo>) -> sortie::Result<()> {
    Ok(())
}

#[sortie::mission]
pub async fn start_first(_start: Start, _again: Start) -> sortie::Result<()> {
    Ok(())
}

#[sortie::mission]
pub async fn wrong_result(_ctx: Ctx<Demo>, _start: Start) -> Result<(), String> {
    Ok(())
}

#[sortie::mission]
pub async fn wrong_value(_ctx: Ctx<Demo>, _start: Start) -> Result<u32, sortie::Error> {
    Ok(1)
}

#[sortie::mission]
pub async fn unit(_ctx: Ctx<Demo>, _start: Start) -> () {}

fn main() {}
