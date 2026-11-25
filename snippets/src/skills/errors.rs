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

/// An optional command: retried once on a timeout, logged, never the end of the mission.
pub async fn optional(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    let sam = ctx.group(groups::SAM_NORTH)?;
    // snippet:start skills-errors-optional
    let alarm = || sam.set_option(Alarm::Red).key("sam-north-red");
    let mut done = alarm().await;
    if let Err(sortie::Error::Timeout(_)) = &done {
        // The same key: a first try that reached DCS after all does not run twice.
        done = alarm().await;
    }
    if let Err(e) = done {
        // Optional: note it and go on instead of ending the mission function.
        eprintln!("SAM North stays as it was: {}", e.report());
    }
    // snippet:end skills-errors-optional
    Ok(())
}

/// A read refused by a limit, sent again after the wait the engine names.
pub async fn limited(ctx: Ctx<Frontline>) -> sortie::Result<()> {
    use sortie::sel::UnitSel;
    // snippet:start skills-errors-limit
    let mut tries = 0;
    let fuel = loop {
        match ctx.dcs().unit(UnitSel::name("SAM North-1")).fuel().await {
            Ok(f) => break Some(f),
            // A rate or DCS-time budget: the engine names the wait of this same read.
            Err(e) if tries < 3 && e.retry_after().is_some() => {
                tries += 1;
                ctx.sleep(e.retry_after().unwrap_or_default()).await;
            }
            // A forbid (not in the catalog, switched off): waiting never helps.
            Err(e) if e.is_forbidden() => break None,
            Err(e) => return Err(e),
        }
    };
    // snippet:end skills-errors-limit
    let _ = fuel;
    Ok(())
}
