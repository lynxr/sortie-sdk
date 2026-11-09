//! Examples of `sortie-mission-logic/references/errors.md`.

use super::frontline::{Frontline, groups};
use sortie::prelude::*;

/// Handling the errors a mission expects, passing on the others.
pub async fn handle(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    // snippet:start skills-errors-match
    let sam = match ctx.group(groups::SAM_NORTH) {
        Ok(g) => g,
        // In the manifest, but no live unit now (late activation, already destroyed).
        Err(sortie::Error::NotInWorld { .. }) => return Ok(()),
        // UnknownSymbol: the running mission was built from another scenario lock.
        Err(e) => return Err(e),
    };
    match sam.set_option(Alarm::Red).key("sam-hot").await {
        Ok(()) => {}
        // The group died between the check and the command: expected in a live mission.
        Err(sortie::Error::Gone(f)) => eprintln!("SAM North is gone: {}", f.message),
        // The session ended while the command was in flight: not a logic error.
        Err(e) if e.is_session_ended() => return Ok(()),
        // Anything else: the runtime records it with `e.report()` (code, call site, hint).
        Err(e) => return Err(e),
    }
    // snippet:end skills-errors-match
    Ok(())
}
