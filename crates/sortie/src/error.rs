//! SDK errors. Every variant has a stable code ([`Error::code`]) such as `sortie::gone`, and
//! every error that comes from a command carries the Rust call site that built it (captured
//! with `#[track_caller]`), so a failure points at the line of mission code that caused it.
//!
//! `Display` is one line: ``error[sortie::gone]: `push_task` on group "Red CAP 1": Unit doesn't
//! exist (at logic/src/lib.rs:48:9)``. [`Error::report`] renders the long form of the docs
//! ("Понятные ошибки"), with the Lua traceback, the session and command ids and a hint; the
//! runtime uses it for errors it records. `Error` also implements [`miette::Diagnostic`].

use std::fmt;

use sortie_proto::v1;
use sortie_types::{NotYet, SymbolKind};

/// `sortie::Result<T>`.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Details of a failed command, as reported by the engine or DCS.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Failure {
    /// SDK operation, e.g. `"push_task"`.
    pub op: &'static str,
    /// What the command was aimed at, e.g. `group "Red CAP 1"` (empty if nothing specific).
    pub target: String,
    /// Command id (empty when the failure did not come from a command).
    pub command_id: String,
    /// Session id (empty outside a session).
    pub session: String,
    pub message: String,
    pub lua_traceback: Option<String>,
    /// Rust call site that created the command (`file:line:column`).
    pub origin: Option<String>,
}

impl Failure {
    #[must_use]
    pub fn new(op: &'static str, message: impl Into<String>) -> Self {
        Self {
            op,
            message: message.into(),
            ..Self::default()
        }
    }

    #[must_use]
    pub(crate) fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = target.into();
        self
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}`", self.op)?;
        if !self.target.is_empty() {
            write!(f, " on {}", self.target)?;
        }
        write!(f, ": {}", self.message)?;
        if let Some(origin) = &self.origin {
            write!(f, " (at {origin})")?;
        }
        Ok(())
    }
}

/// Everything that can go wrong in mission logic. Command failures carry a boxed [`Failure`]
/// (match with `Err(Error::Gone(f)) => f.origin...`).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// The target object no longer exists in DCS (died, despawned, Eid of a past life).
    Gone(Box<Failure>),
    /// DCS (or the engine's scheduler) did not complete the command within its timeout.
    Timeout(Box<Failure>),
    /// The Lua layer raised an error; `lua_traceback` says where.
    Lua(Box<Failure>),
    /// The bridge caught a panic, or the engine produced an op the bridge does not know.
    BridgeFault(Box<Failure>),
    /// The engine refused the command.
    Rejected(Box<Failure>),
    /// The session ended before the command completed.
    SessionEnded(Box<Failure>),
    /// The command does not make sense (wrong target kind, missing argument, dev-mode rule).
    Invalid(Box<Failure>),
    /// The engine cannot run this command right now (or does not implement it yet).
    Unavailable(Box<Failure>),
    /// A live group or unit already has this name.
    NameConflict(Box<Failure>),
    /// The simulation is paused and the command asked to fail instead of waiting.
    Paused(Box<Failure>),
    /// This kind of command is disabled after it crashed DCS or faulted the bridge.
    Quarantined(Box<Failure>),
    /// A compare-and-swap write lost against another writer.
    Conflict(Box<Failure>),
    /// The manifest of the running mission has no such symbol (schema drift).
    UnknownSymbol {
        kind: &'static str,
        ident: String,
        origin: Option<String>,
    },
    /// The symbol exists in the manifest but not (or no longer) in the world.
    NotInWorld {
        kind: &'static str,
        ident: String,
        origin: Option<String>,
    },
    /// A state document could not be read or written.
    State { name: String, message: String },
    /// The connection to the engine failed or was lost (gRPC transport, closed stream).
    Transport(String),
    /// A feature that lands in a later milestone.
    NotYet(NotYet),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error[{}]: ", self.code())?;
        let at = |f: &mut fmt::Formatter<'_>, origin: &Option<String>| match origin {
            Some(o) => write!(f, " (at {o})"),
            None => Ok(()),
        };
        match self {
            Error::UnknownSymbol {
                kind,
                ident,
                origin,
            } => {
                write!(
                    f,
                    "the running mission has no {kind} `{ident}`; regenerate the scenario lock"
                )?;
                at(f, origin)
            }
            Error::NotInWorld {
                kind,
                ident,
                origin,
            } => {
                write!(f, "{kind} `{ident}` is not in the world right now")?;
                at(f, origin)
            }
            Error::State { name, message } => write!(f, "state `{name}`: {message}"),
            Error::Transport(message) => write!(f, "engine connection: {message}"),
            Error::NotYet(n) => write!(f, "{n}"),
            other => match other.failure() {
                Some(failure) => write!(f, "{failure}"),
                None => Ok(()),
            },
        }
    }
}

impl Error {
    /// Maps an engine `CommandError` for operation `op`.
    #[must_use]
    pub fn from_command_error(op: &'static str, failure: Failure, e: &v1::CommandError) -> Self {
        let failure = Failure {
            op,
            message: e.message.clone(),
            lua_traceback: (!e.lua_traceback.is_empty()).then(|| e.lua_traceback.clone()),
            ..failure
        };
        Self::with_code(
            v1::ErrorCode::try_from(e.code).unwrap_or(v1::ErrorCode::Unspecified),
            failure,
        )
    }

    /// The variant for an engine error code.
    #[must_use]
    pub fn with_code(code: v1::ErrorCode, failure: Failure) -> Self {
        match code {
            v1::ErrorCode::Gone => Error::Gone(Box::new(failure)),
            v1::ErrorCode::Timeout => Error::Timeout(Box::new(failure)),
            v1::ErrorCode::LuaError => Error::Lua(Box::new(failure)),
            v1::ErrorCode::BridgeFault => Error::BridgeFault(Box::new(failure)),
            v1::ErrorCode::Rejected => Error::Rejected(Box::new(failure)),
            v1::ErrorCode::SessionEnded => Error::SessionEnded(Box::new(failure)),
            v1::ErrorCode::Unavailable => Error::Unavailable(Box::new(failure)),
            v1::ErrorCode::NameConflict => Error::NameConflict(Box::new(failure)),
            v1::ErrorCode::Paused => Error::Paused(Box::new(failure)),
            v1::ErrorCode::Quarantined => Error::Quarantined(Box::new(failure)),
            v1::ErrorCode::Conflict => Error::Conflict(Box::new(failure)),
            v1::ErrorCode::Invalid | v1::ErrorCode::Unspecified => {
                Error::Invalid(Box::new(failure))
            }
        }
    }

    /// Unknown symbol of `kind`.
    #[must_use]
    pub fn unknown_symbol(kind: SymbolKind, ident: &str) -> Self {
        Error::UnknownSymbol {
            kind: kind.as_str(),
            ident: ident.to_owned(),
            origin: None,
        }
    }

    /// Stable machine-readable code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Error::Gone(_) => "sortie::gone",
            Error::Timeout(_) => "sortie::timeout",
            Error::Lua(_) => "sortie::lua_error",
            Error::BridgeFault(_) => "sortie::bridge_fault",
            Error::Rejected(_) => "sortie::rejected",
            Error::SessionEnded(_) => "sortie::session_ended",
            Error::Invalid(_) => "sortie::invalid",
            Error::Unavailable(_) => "sortie::unavailable",
            Error::NameConflict(_) => "sortie::name_conflict",
            Error::Paused(_) => "sortie::paused",
            Error::Quarantined(_) => "sortie::quarantined",
            Error::Conflict(_) => "sortie::conflict",
            Error::UnknownSymbol { .. } => "sortie::unknown_symbol",
            Error::NotInWorld { .. } => "sortie::not_in_world",
            Error::State { .. } => "sortie::state",
            Error::Transport(_) => "sortie::transport",
            Error::NotYet(_) => "sortie::not_yet",
        }
    }

    /// The command failure behind this error, if it came from a command.
    #[must_use]
    pub fn failure(&self) -> Option<&Failure> {
        match self {
            Error::Gone(f)
            | Error::Timeout(f)
            | Error::Lua(f)
            | Error::BridgeFault(f)
            | Error::Rejected(f)
            | Error::SessionEnded(f)
            | Error::Invalid(f)
            | Error::Unavailable(f)
            | Error::NameConflict(f)
            | Error::Paused(f)
            | Error::Quarantined(f)
            | Error::Conflict(f) => Some(f.as_ref()),
            _ => None,
        }
    }

    /// The Rust call site (`file:line:column`) that produced the failing command or lookup.
    #[must_use]
    pub fn origin(&self) -> Option<&str> {
        match self {
            Error::UnknownSymbol { origin, .. } | Error::NotInWorld { origin, .. } => {
                origin.as_deref()
            }
            other => other.failure().and_then(|f| f.origin.as_deref()),
        }
    }

    /// The Lua traceback, for errors raised by the Lua layer.
    #[must_use]
    pub fn lua_traceback(&self) -> Option<&str> {
        self.failure().and_then(|f| f.lua_traceback.as_deref())
    }

    /// `true` for [`Error::SessionEnded`]: the normal fate of work still running when the
    /// session ends (the runtime does not record it as a logic error).
    #[must_use]
    pub fn is_session_ended(&self) -> bool {
        matches!(self, Error::SessionEnded(_))
    }

    /// A hint for the person reading the error.
    #[must_use]
    pub fn help(&self) -> Option<&'static str> {
        Some(match self {
            Error::Gone(_) => {
                "the object no longer exists in DCS (it died or despawned before the command \
                 ran); match on `Error::Gone` to handle it"
            }
            Error::Timeout(_) => {
                "DCS did not answer in time: raise `.timeout(..)`, or check `sortie top` for a \
                 busy pump"
            }
            Error::Lua(_) => "the Lua layer raised an error; the traceback shows where",
            Error::BridgeFault(_) => {
                "the bridge caught a fault; see Logs/sortie-bridge.log and report it"
            }
            Error::Rejected(_) => "the engine refused the command; the message says why",
            Error::SessionEnded(_) => {
                "the session ended first; the next session runs the mission function again"
            }
            Error::Invalid(_) => {
                "the command does not fit its target or misses an argument (dev mode also \
                 requires `.key(..)` on spawns and destroys)"
            }
            Error::Unavailable(_) => "the engine cannot run this command right now",
            Error::NameConflict(_) => {
                "a live group already has this name; drop `.named(..)` or choose another name"
            }
            Error::Paused(_) => {
                "the simulation is paused and the command was sent with `.on_pause(Fail)`"
            }
            Error::Quarantined(_) => {
                "this kind of command is disabled after it faulted; run `sortie doctor`"
            }
            Error::Conflict(_) => "another writer changed the document; reload and retry",
            Error::UnknownSymbol { .. } => {
                "the running mission was generated from another scenario lock: run \
                 `sortie-gen lock`, rebuild the logic and the mission"
            }
            Error::NotInWorld { .. } => {
                "the symbol is in the manifest but has no live object (late activation, or \
                 already destroyed)"
            }
            Error::State { .. } => "the engine's state store refused the operation",
            Error::Transport(_) => {
                "is `sortied` running, and does SORTIE_ENGINE point at its gRPC address?"
            }
            Error::NotYet(_) => return None,
        })
    }

    /// The long, multi-line form shown by the runtime and `sortie::main!`:
    ///
    /// ```text
    /// error[sortie::gone]: `push_task` on group "Red CAP 1": Unit doesn't exist
    ///   --> logic/src/lib.rs:48:9
    ///    | Lua: Scripts/Sortie/layer/exec/set_task.lua:41: Unit doesn't exist
    ///    | session 01JC6Z… · command 0001-fake01-12
    ///  help: the object no longer exists in DCS (...)
    /// ```
    #[must_use]
    pub fn report(&self) -> String {
        use fmt::Write as _;
        let mut out = format!("error[{}]: ", self.code());
        match self.failure() {
            Some(f) => {
                let _ = write!(out, "`{}`", f.op);
                if !f.target.is_empty() {
                    let _ = write!(out, " on {}", f.target);
                }
                let _ = write!(out, ": {}", f.message);
            }
            None => {
                let line = self.to_string();
                let line = line
                    .split_once("]: ")
                    .map_or(line.as_str(), |(_, rest)| rest);
                // The origin goes on its own line below.
                let line = match self.origin() {
                    Some(o) => line.trim_end_matches(&format!(" (at {o})")),
                    None => line,
                };
                out.push_str(line);
            }
        }
        if let Some(origin) = self.origin() {
            let _ = write!(out, "\n  --> {origin}");
        }
        if let Some(f) = self.failure() {
            if let Some(tb) = &f.lua_traceback {
                for line in tb.lines() {
                    let _ = write!(out, "\n   | Lua: {line}");
                }
            }
            let mut ids = Vec::new();
            if !f.session.is_empty() {
                ids.push(format!("session {}", f.session));
            }
            if !f.command_id.is_empty() {
                ids.push(format!("command {}", f.command_id));
            }
            if !ids.is_empty() {
                let _ = write!(out, "\n   | {}", ids.join(" · "));
            }
        }
        if let Some(help) = self.help() {
            let _ = write!(out, "\n help: {help}");
        }
        out
    }
}

impl miette::Diagnostic for Error {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new(Error::code(self)))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Error::help(self).map(|h| Box::new(h) as Box<dyn fmt::Display>)
    }
}

impl From<NotYet> for Error {
    fn from(n: NotYet) -> Self {
        Error::NotYet(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gone() -> Error {
        let e = v1::CommandError {
            code: v1::ErrorCode::Gone as i32,
            message: "Unit doesn't exist".into(),
            lua_traceback: "layer/exec/set_task.lua:41: Unit doesn't exist".into(),
        };
        let base = Failure {
            target: "group \"Red CAP 1\"".into(),
            command_id: "0001-fake01-12".into(),
            session: "01SORTIE000000000000000001".into(),
            origin: Some("logic/src/lib.rs:48:9".into()),
            ..Failure::default()
        };
        Error::from_command_error("push_task", base, &e)
    }

    #[test]
    fn command_errors_map_to_variants_with_their_call_site() {
        let err = gone();
        assert_eq!(err.code(), "sortie::gone");
        assert_eq!(
            err.lua_traceback(),
            Some("layer/exec/set_task.lua:41: Unit doesn't exist")
        );
        assert_eq!(err.origin(), Some("logic/src/lib.rs:48:9"));
        assert_eq!(
            err.to_string(),
            "error[sortie::gone]: `push_task` on group \"Red CAP 1\": Unit doesn't exist \
             (at logic/src/lib.rs:48:9)"
        );
        for code in [
            v1::ErrorCode::Timeout,
            v1::ErrorCode::SessionEnded,
            v1::ErrorCode::Rejected,
            v1::ErrorCode::LuaError,
        ] {
            let e = Error::with_code(code, Failure::new("message", "x"));
            assert!(e.failure().is_some(), "{e:?}");
        }
    }

    #[test]
    fn report_has_the_documented_shape() {
        let report = gone().report();
        let lines: Vec<&str> = report.lines().collect();
        assert_eq!(
            lines[0],
            "error[sortie::gone]: `push_task` on group \"Red CAP 1\": Unit doesn't exist"
        );
        assert_eq!(lines[1], "  --> logic/src/lib.rs:48:9");
        assert_eq!(
            lines[2],
            "   | Lua: layer/exec/set_task.lua:41: Unit doesn't exist"
        );
        assert_eq!(
            lines[3],
            "   | session 01SORTIE000000000000000001 · command 0001-fake01-12"
        );
        assert!(lines[4].starts_with(" help: the object no longer exists"));
    }

    #[test]
    fn lookup_errors_carry_their_origin() {
        let e = Error::NotInWorld {
            kind: "group",
            ident: "SAM_NORTH".into(),
            origin: Some("src/lib.rs:13:19".into()),
        };
        assert_eq!(
            e.to_string(),
            "error[sortie::not_in_world]: group `SAM_NORTH` is not in the world right now \
             (at src/lib.rs:13:19)"
        );
        let report = e.report();
        assert!(report.starts_with(
            "error[sortie::not_in_world]: group `SAM_NORTH` is not in the world right now\n  --> src/lib.rs:13:19"
        ));
        let t = Error::Transport("connection refused".into());
        assert_eq!(t.code(), "sortie::transport");
        assert!(t.report().contains("help: is `sortied` running"));
    }
}
