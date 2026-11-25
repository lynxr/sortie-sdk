//! SDK errors. Every variant has a stable code ([`Error::code`]) such as `sortie::gone`, and
//! every error that comes from a command carries the Rust call site that built it (captured
//! with `#[track_caller]`), so a failure points at the line of mission code that caused it.
//!
//! `Display` is one line: ``error[sortie::gone]: `push_task` on group "Red CAP 1": Unit doesn't
//! exist (at logic/src/lib.rs:48:9)``. [`Error::report`] renders the long form of the docs
//! ("Понятные ошибки"), with the Lua traceback, the session and command ids and a hint; the
//! runtime uses it for errors it records. `Error` also implements [`miette::Diagnostic`].

use std::fmt;
use std::time::Duration;

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
    /// The capability the refusal names (`CommandError.capability`: missing, switched off or
    /// quarantined), empty when none; see [`Error::capability`].
    pub capability: String,
    /// The limit that refused the command, or what it waited for until its deadline
    /// (`CommandError.limit`, engine 0.4.0, API revision 3); see [`Error::limit`]. `None` for
    /// every other failure and from an older engine.
    pub limit: Option<Limit>,
}

/// Which of the engine's limits refused a command, or what the command waited for until its
/// deadline (`CommandError.limit`; engine 0.4.0, API revision 3). Read it with
/// [`Error::limit`]; a logic tells the cause from [`Limit::kind`] and [`Limit::budget`]
/// without parsing the message.
///
/// The numbers are whole, in the budget's unit: calls (`call_rate.*`, `batch.calls`),
/// microseconds of DCS time (`dcs_time.*`), bytes (`request.bytes`), handles (`handles.*`),
/// slow calls in 10 minutes (`bridge.slow_calls`); 0 where a number does not apply (the
/// waits).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Limit {
    pub kind: LimitKind,
    /// The budget's stable name: `"call_rate.read"`, `"dcs_time.read"`, `"dcs_time.heavy"`,
    /// `"batch.calls"`, `"handles.created"`, `"delay.tasking"`, ... (the docs list them).
    pub budget: String,
    /// Whose budget: the writer (the API token's name, else the logic's) for `call_rate.*`
    /// and `dcs_time.*`; empty for a limit of the session or of DCS.
    pub writer: String,
    /// What the budget holds at most.
    pub capacity: i64,
    /// What it refills a second (0: it does not refill).
    pub per_second: i64,
    /// What this command needed from it.
    pub needed: i64,
    /// What it held when the command was refused (below 0: a debt).
    pub available: i64,
    /// When this same command, sent again unchanged, passes every rate budget it needs, if
    /// the logic sends nothing else meanwhile; `None` where no refill can tell (a size, a cap,
    /// a wait, a heavy call while the previous one is not back). See [`Error::retry_after`].
    pub retry_after: Option<Duration>,
}

/// The kind of a [`Limit`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LimitKind {
    /// A kind this SDK does not know (a newer engine), or none.
    #[default]
    Unknown,
    /// Calls per second of one class (`call_rate.read` 200, `.mutate` 50, `.heavy` 2,
    /// `.admin` 1): [`Error::Rejected`].
    CallRate,
    /// DCS time per second (`dcs_time.read`, `dcs_time.heavy`): [`Error::Rejected`]. Also a
    /// heavy call sent while the logic's previous heavy call has not come back
    /// (`dcs_time.heavy`, no [`Limit::retry_after`]).
    DcsTime,
    /// Calls of one batch (`batch.calls`: 64, 256 reads): [`Error::Invalid`].
    BatchSize,
    /// Bytes of one call or batch (`request.bytes`: 64 KiB): [`Error::Invalid`].
    RequestSize,
    /// Objects made by calls alive in the session (`handles.created`: 256):
    /// [`Error::Rejected`].
    CreatedHandles,
    /// A handle the Lua layer dropped as the least recently used of its 1024 read handles
    /// (`handles.read`): [`Error::Gone`]; read the object again.
    ReadHandles,
    /// A catalog path refused until DCS restarts after three calls slower than the bridge's
    /// 1 s watchdog in 10 minutes (`bridge.slow_calls`): [`Error::Quarantined`].
    SlowCalls,
    /// The waits ([`LimitKind::is_wait`]; the command never ran in DCS), which end at the
    /// deadline ([`Error::Timeout`]): the spawn gate (`gate.spawn`, `gate.warmup`), the spawns
    /// in flight (`window.spawns`), the tasking delay after a spawn (`delay.tasking`), one heavy
    /// call at a time (`slot.heavy`), the link's credits (`link.credits`), the link was down
    /// (`link.down`), the `after` command (`wait.after`), the engine's queue (`wait.queue`),
    /// the bridge did not start it (`bridge.deadline`).
    ///
    /// [`LimitKind::After`] also ends when the `after` command failed: the error then has that
    /// command's code (and capability), [`Error::Rejected`] for a refused one, and the same
    /// command sent again fails the same way at once (the engine remembers that failure). Send
    /// the `after` command again, then this one after the new one.
    SpawnGate,
    SpawnWindow,
    TaskingDelay,
    HeavySlot,
    LinkCredits,
    LinkDown,
    After,
    Queue,
    BridgeDeadline,
}

impl LimitKind {
    fn of(kind: i32) -> Self {
        use v1::LimitKind as K;
        match K::try_from(kind).unwrap_or(K::Unspecified) {
            K::Unspecified => LimitKind::Unknown,
            K::CallRate => LimitKind::CallRate,
            K::DcsTime => LimitKind::DcsTime,
            K::BatchSize => LimitKind::BatchSize,
            K::RequestSize => LimitKind::RequestSize,
            K::CreatedHandles => LimitKind::CreatedHandles,
            K::ReadHandles => LimitKind::ReadHandles,
            K::SlowCalls => LimitKind::SlowCalls,
            K::SpawnGate => LimitKind::SpawnGate,
            K::SpawnWindow => LimitKind::SpawnWindow,
            K::TaskingDelay => LimitKind::TaskingDelay,
            K::HeavySlot => LimitKind::HeavySlot,
            K::LinkCredits => LimitKind::LinkCredits,
            K::LinkDown => LimitKind::LinkDown,
            K::After => LimitKind::After,
            K::Queue => LimitKind::Queue,
            K::BridgeDeadline => LimitKind::BridgeDeadline,
        }
    }

    /// A wait: the command never ran in DCS. It ended at the deadline ([`Error::Timeout`]),
    /// except [`LimitKind::After`] when the `after` command failed: then the error has that
    /// command's code, and sending the same command again does not help.
    #[must_use]
    pub fn is_wait(self) -> bool {
        matches!(
            self,
            LimitKind::SpawnGate
                | LimitKind::SpawnWindow
                | LimitKind::TaskingDelay
                | LimitKind::HeavySlot
                | LimitKind::LinkCredits
                | LimitKind::LinkDown
                | LimitKind::After
                | LimitKind::Queue
                | LimitKind::BridgeDeadline
        )
    }
}

impl Limit {
    /// The SDK's form of `CommandError.limit`.
    #[must_use]
    pub fn from_proto(l: &v1::Limit) -> Self {
        Self {
            kind: LimitKind::of(l.kind),
            budget: l.budget.clone(),
            writer: l.writer.clone(),
            capacity: l.capacity,
            per_second: l.per_second,
            needed: l.needed,
            available: l.available,
            retry_after: (l.retry_after_ms > 0)
                .then(|| Duration::from_millis(u64::from(l.retry_after_ms))),
        }
    }
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
    /// A keyed universal call was answered from the key store, which kept no values for it:
    /// the call ran under this key before, it was not run again, and its result was too large
    /// to keep (above 64 KiB). Read what you need with a call of its own.
    ResultNotKept(Box<Failure>),
    /// The operation needs a newer engine (`Start.api_rev` is below the API revision that
    /// introduced it); nothing was sent.
    EngineTooOld(Box<Failure>),
    /// The operation belongs to an API revision above the one the logic declares
    /// (`#[sortie::mission(api_rev = N)]`); nothing was sent.
    Revision(Box<Failure>),
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
            capability: e.capability.clone(),
            limit: e.limit.as_deref().map(Limit::from_proto),
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
            Error::ResultNotKept(_) => "sortie::result_not_kept",
            Error::EngineTooOld(_) => "sortie::engine_too_old",
            Error::Revision(_) => "sortie::revision",
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
            | Error::Conflict(f)
            | Error::ResultNotKept(f)
            | Error::EngineTooOld(f)
            | Error::Revision(f) => Some(f.as_ref()),
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

    /// The capability a refusal names (`UNAVAILABLE`, `REJECTED` by a switch, `QUARANTINED`):
    /// for example `"dcs_call"` or `"call:mission:land.getHeight"`; compare with the
    /// constants of [`crate::caps`] and look it up in `ctx.capabilities()`.
    #[must_use]
    pub fn capability(&self) -> Option<&str> {
        self.failure()
            .map(|f| f.capability.as_str())
            .filter(|c| !c.is_empty())
    }

    /// The limit that refused the command, or what it waited for until its deadline (engine
    /// 0.4.0; `None` from an older engine and for every other failure). The code stays what it
    /// was: [`Error::Rejected`] for a rate, the DCS time or the created handles,
    /// [`Error::Invalid`] for a batch or request size, [`Error::Gone`] for a dropped read
    /// handle, [`Error::Quarantined`] for a path the bridge refuses after slow calls,
    /// [`Error::Timeout`] for a wait (the command never ran in DCS), except [`LimitKind::After`]
    /// when the `after` command failed: then the code is that command's.
    ///
    /// ```ignore
    /// match ctx.dcs().timer().time().await {
    ///     Err(e) if e.limit().is_some_and(|l| l.kind == LimitKind::DcsTime) => { /* .. */ }
    ///     other => { /* .. */ }
    /// }
    /// ```
    #[must_use]
    pub fn limit(&self) -> Option<&Limit> {
        self.failure().and_then(|f| f.limit.as_ref())
    }

    /// When the same command, sent again unchanged, passes the limit that refused it
    /// ([`Limit::retry_after`]): `Some` for the rate budgets (`call_rate.*`, `dcs_time.*`),
    /// counted from the refusal, if the logic sends nothing else meanwhile and no earlier
    /// command's DCS time is charged meanwhile. `None` for a heavy call refused because the
    /// logic's previous heavy call has not come back (`dcs_time.heavy`; its cost is not known
    /// yet): send it again once that one answered. The SDK never retries by itself.
    #[must_use]
    pub fn retry_after(&self) -> Option<Duration> {
        self.limit().and_then(|l| l.retry_after)
    }

    /// One of the engine's limits refused the command, or it waited until its deadline
    /// ([`Error::limit`] is `Some`).
    #[must_use]
    pub fn is_limit(&self) -> bool {
        self.limit().is_some()
    }

    /// A forbid, which no retry helps: the engine refused the command by policy, naming the
    /// capability ([`Error::Rejected`] with [`Error::capability`] and no [`Error::limit`]): a
    /// catalog path that is not in the catalog, that is off by default or that the owner
    /// switched off, an operation the owner disabled, a guarded argument (`admin.addresses`),
    /// a player's object the guard keeps. A rate refusal names no capability, so it is never
    /// a forbid; against an engine before 0.4.0, the Lua layer's refusal of a create past its
    /// 256 handles names one and carries no limit, so it reads as a forbid there.
    #[must_use]
    pub fn is_forbidden(&self) -> bool {
        matches!(self, Error::Rejected(_)) && self.capability().is_some() && !self.is_limit()
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
                "the deadline passed: with `Error::limit()` the command waited and never ran in \
                 DCS (the limit names what for), else it was sent and may still run; a retry \
                 with the same `.key(..)` never runs it twice. Raise `.timeout(..)`, or check \
                 `sortie top` for a busy pump"
            }
            Error::Lua(_) => "the Lua layer raised an error; the traceback shows where",
            Error::BridgeFault(_) => {
                "the bridge caught a fault; see Logs/sortie-bridge.log and report it"
            }
            Error::Rejected(_) => {
                "the engine refused the command: `Error::limit()` names a limit (wait \
                 `Error::retry_after()` and send it again), `Error::is_forbidden()` a forbid \
                 (no retry helps); the message says more"
            }
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
                "this kind of command is quarantined: it was running when DCS crashed, or it \
                 faulted the bridge; `Error::capability()` names it, the engine owner releases \
                 it (`sortie caps --release <name>`)"
            }
            Error::Conflict(_) => "another writer changed the document; reload and retry",
            Error::ResultNotKept(_) => {
                "the call ran under this key before and was not run again; the engine keeps a \
                 result above 64 KiB without its values, so read what you need with a call of \
                 its own"
            }
            Error::EngineTooOld(_) => {
                "the engine is older than this operation: update sortied, or keep to the \
                 operations of its API revision (`ctx.engine_api_rev()`)"
            }
            Error::Revision(_) => {
                "declare the API revision the operation belongs to: \
                 `#[sortie::mission(api_rev = N)]` (a logic's revision is written in its source)"
            }
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
            capability: String::new(),
            limit: None,
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
    fn no_help_text_has_a_run_of_spaces() {
        let f = || Box::new(Failure::new("x", "m"));
        let all = [
            Error::Gone(f()),
            Error::Timeout(f()),
            Error::Lua(f()),
            Error::BridgeFault(f()),
            Error::Rejected(f()),
            Error::SessionEnded(f()),
            Error::Invalid(f()),
            Error::Unavailable(f()),
            Error::NameConflict(f()),
            Error::Paused(f()),
            Error::Quarantined(f()),
            Error::Conflict(f()),
            Error::ResultNotKept(f()),
            Error::EngineTooOld(f()),
            Error::Revision(f()),
            Error::UnknownSymbol {
                kind: "group",
                ident: "X".into(),
                origin: None,
            },
            Error::NotInWorld {
                kind: "group",
                ident: "X".into(),
                origin: None,
            },
            Error::State {
                name: "n".into(),
                message: "m".into(),
            },
            Error::Transport("t".into()),
        ];
        for e in &all {
            let help = e.help().unwrap_or_default();
            assert!(!help.contains("  "), "{}: {help:?}", e.code());
        }
    }

    fn refusal(code: v1::ErrorCode, capability: &str, limit: Option<v1::Limit>) -> Error {
        let e = v1::CommandError {
            code: code as i32,
            message: "m".into(),
            lua_traceback: String::new(),
            capability: capability.into(),
            limit: limit.map(Box::new),
        };
        Error::from_command_error("dcs_call", Failure::default(), &e)
    }

    /// 0.4.0: a limit refusal is told by data, not by its text: `limit()`, `retry_after()`,
    /// `is_limit()`; a forbid (a capability and no limit) by `is_forbidden()`. The variant
    /// and the capability stay those of 0.3.0.
    #[test]
    fn a_limit_is_read_without_parsing_the_message() {
        let rate = refusal(
            v1::ErrorCode::Rejected,
            "",
            Some(v1::Limit {
                kind: v1::LimitKind::CallRate as i32,
                budget: "call_rate.read".into(),
                writer: "w".into(),
                capacity: 200,
                per_second: 200,
                needed: 200,
                available: -56,
                retry_after_ms: 1280,
            }),
        );
        assert!(matches!(rate, Error::Rejected(_)));
        assert!(rate.is_limit() && !rate.is_forbidden());
        let l = rate.limit().unwrap();
        assert_eq!(l.kind, LimitKind::CallRate);
        assert_eq!(
            (l.budget.as_str(), l.writer.as_str()),
            ("call_rate.read", "w")
        );
        assert_eq!(
            (l.capacity, l.per_second, l.needed, l.available),
            (200, 200, 200, -56)
        );
        assert_eq!(rate.retry_after(), Some(Duration::from_millis(1280)));
        // A wait: a TIMEOUT that never ran; no retry time.
        let waited = refusal(
            v1::ErrorCode::Timeout,
            "",
            Some(v1::Limit {
                kind: v1::LimitKind::TaskingDelay as i32,
                budget: "delay.tasking".into(),
                ..Default::default()
            }),
        );
        assert_eq!(waited.limit().unwrap().kind, LimitKind::TaskingDelay);
        assert!(waited.limit().unwrap().kind.is_wait());
        assert_eq!(waited.retry_after(), None);
        // A forbid: REJECTED with a capability and no limit; never a limit.
        let off = refusal(
            v1::ErrorCode::Rejected,
            "call:mission:Airbase.setCoalition",
            None,
        );
        assert!(off.is_forbidden() && !off.is_limit());
        assert_eq!(off.retry_after(), None);
        assert_eq!(off.capability(), Some("call:mission:Airbase.setCoalition"));
        // The layer's created-handle cap keeps the path's capability and is a limit.
        let cap = refusal(
            v1::ErrorCode::Rejected,
            "call:mission:Spot.createLaser",
            Some(v1::Limit {
                kind: v1::LimitKind::CreatedHandles as i32,
                budget: "handles.created".into(),
                capacity: 256,
                ..Default::default()
            }),
        );
        assert!(cap.is_limit() && !cap.is_forbidden());
        assert_eq!(cap.capability(), Some("call:mission:Spot.createLaser"));
        // A kind this SDK does not know (a newer engine) keeps its budget's name.
        let newer = refusal(
            v1::ErrorCode::Rejected,
            "",
            Some(v1::Limit {
                kind: 999,
                budget: "something.new".into(),
                ..Default::default()
            }),
        );
        assert_eq!(newer.limit().unwrap().kind, LimitKind::Unknown);
        assert_eq!(newer.limit().unwrap().budget, "something.new");
        // A QUARANTINED path is no forbid (it is not REJECTED), nor a GONE.
        assert!(!refusal(v1::ErrorCode::Quarantined, "call:mission:x.y", None).is_forbidden());
        assert!(!gone().is_forbidden() && !gone().is_limit());
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
