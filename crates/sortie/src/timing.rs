//! What the commands of this logic cost: [`crate::Ctx::take_command_timings`].
//!
//! The SDK keeps one [`CommandTiming`] per command whose result arrived (an answer from the
//! engine, an error included; not a command the SDK refused before sending it, nor one cut off
//! by the end of the session or a lost connection), at most [`KEPT_TIMINGS`] per session.

use std::collections::VecDeque;
use std::time::Duration;

/// How many timings a session keeps until they are taken; the oldest go first.
pub const KEPT_TIMINGS: usize = 4096;

/// One command as this logic saw it.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandTiming {
    /// The SDK operation: `"dcs_call"`, `"dcs_batch"`, `"spawn_static"`, `"draw"`, ...
    pub op: &'static str,
    /// What it was aimed at, as error messages name it: the Lua path of a catalog call, a
    /// group, a resource; empty when the operation has no single target.
    pub target: String,
    /// The command id ([`crate::Cmd::id`]).
    pub command_id: String,
    /// The engine answered without an error.
    pub ok: bool,
    /// Time spent inside DCS on the command, microseconds (`CommandResult.cost_us`). A batch
    /// counts all its calls. 0 when nothing ran in DCS: a replay by key, a refusal, a result
    /// the engine answered itself.
    pub cost_us: u32,
    /// From sending the command to its result: the engine's queue and limits, the wait for
    /// the next mission pump, the link both ways.
    pub round_trip: Duration,
    /// Model time of the result, seconds.
    pub model_time: f64,
}

/// The timings taken by [`crate::Ctx::take_command_timings`], oldest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandTimings {
    pub commands: Vec<CommandTiming>,
    /// Timings dropped since the last take because more than [`KEPT_TIMINGS`] piled up.
    pub dropped: u64,
}

#[derive(Debug, Default)]
pub(crate) struct Timings {
    kept: VecDeque<CommandTiming>,
    dropped: u64,
}

impl Timings {
    pub(crate) fn push(&mut self, t: CommandTiming) {
        if self.kept.len() == KEPT_TIMINGS {
            self.kept.pop_front();
            self.dropped += 1;
        }
        self.kept.push_back(t);
    }

    pub(crate) fn take(&mut self) -> CommandTimings {
        CommandTimings {
            commands: self.kept.drain(..).collect(),
            dropped: std::mem::take(&mut self.dropped),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing(n: usize) -> CommandTiming {
        CommandTiming {
            op: "dcs_call",
            target: format!("`land.getHeight` {n}"),
            command_id: n.to_string(),
            ok: true,
            cost_us: 10,
            round_trip: Duration::from_millis(20),
            model_time: 1.0,
        }
    }

    #[test]
    fn the_log_keeps_the_newest_and_counts_what_it_dropped() {
        let mut t = Timings::default();
        for n in 0..KEPT_TIMINGS + 3 {
            t.push(timing(n));
        }
        let taken = t.take();
        assert_eq!(taken.dropped, 3);
        assert_eq!(taken.commands.len(), KEPT_TIMINGS);
        assert_eq!(taken.commands[0].command_id, "3");
        // A take empties the log and resets the count.
        assert_eq!(t.take(), CommandTimings::default());
    }
}
