use sortie::prelude::*;

#[sortie::mission]
pub fn run(_ctx: Ctx<Demo>, _start: Start) -> sortie::Result<()> {
    Ok(())
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Demo;

fn main() {}
