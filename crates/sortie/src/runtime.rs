//! The logic runtime: attaches to the engine over a [`LogicConn`] (gRPC `Logic.Attach` or an
//! in-process channel pair), keeps the world replica, routes results/events, and runs the
//! mission function once per session.
//!
//! Single-threaded by design: everything runs on a `current_thread` runtime inside a
//! `tokio::task::LocalSet`, so mission tasks need not be `Send` and runs on tokio's paused clock
//! are reproducible. Nothing here waits for work done on another OS thread.
//!
//! Protocol (see `proto/sortie/v1/logic.proto`):
//! 1. the runtime sends `LogicUp.attach` first;
//! 2. the engine answers `start` (or `rejected`); the runtime builds the replica from the
//!    snapshot, loads zones from the manifest and spawns `mission(ctx, start)`;
//! 3. `tick` → apply `delta`, update model time, wake `wait_until`/`sleep`/timers, let the woken
//!    tasks run until they block again, then `TickDone{n}`; `event` → apply it to the replica
//!    **first**, then wake `wait_until` and hand it to `ctx.events()` / menu presses;
//!    `result`/`state` → complete the waiting futures;
//! 4. `ended` → record the outcome, resolve `until_session_end()`, fail pending commands with
//!    `Error::SessionEnded`, abort the session's tasks; wait for the next `start`;
//! 5. `shutdown` → exit; the stream closing → [`main`] reconnects with backoff (1 s … 30 s).
//!
//! Every task of a session (the mission function and everything started with
//! [`Ctx::spawn`](crate::Ctx::spawn)) is tracked: an `Err` it returns or a panic is recorded
//! with the Rust call site ([`LogicHandle::errors`]), and it is aborted when its session ends.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::process::ExitCode;
use std::rc::{Rc, Weak};
use std::time::Duration;

use futures::FutureExt as _;
use futures::future::LocalBoxFuture;
use sortie_proto::manifest::v1::Manifest;
use sortie_proto::v1::{self, logic_down, logic_up};
use sortie_types::{Outcome, Scenario};
use sortie_world::World;
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use tokio::task::AbortHandle;

use crate::menu::MenuInbox;
use crate::{Ctx, Error, Failure, Result, Start};

/// The per-session entry point. Implemented for every
/// `Fn(Ctx<M>, Start) -> impl Future<Output = sortie::Result<()>> + 'static`, i.e. for every
/// `#[sortie::mission] async fn run(ctx: Ctx<M>, start: Start) -> sortie::Result<()>`.
pub trait Mission<M>: 'static {
    fn start(&self, ctx: Ctx<M>, start: Start) -> LocalBoxFuture<'static, Result<()>>;
}

impl<M, F, Fut> Mission<M> for F
where
    M: Scenario,
    F: Fn(Ctx<M>, Start) -> Fut + 'static,
    Fut: Future<Output = Result<()>> + 'static,
{
    fn start(&self, ctx: Ctx<M>, start: Start) -> LocalBoxFuture<'static, Result<()>> {
        Box::pin(self(ctx, start))
    }
}

/// Messages buffered between the gRPC stream and the logic loop. When the logic stops
/// reading, the buffer fills, gRPC flow control stops the stream and the engine's own bounded
/// queue decides when to detach the logic.
pub const DOWN_BUFFER: usize = 256;

/// The logic side of one Logic stream.
#[derive(Debug)]
pub struct LogicConn {
    pub up: mpsc::UnboundedSender<v1::LogicUp>,
    pub down: mpsc::Receiver<v1::LogicDown>,
}

/// What the runtime announces in `Attach`, and how it behaves.
#[derive(Clone, Debug)]
pub struct AttachOptions {
    /// Logic program name (`Attach.logic`).
    pub logic: String,
    /// Build id of this binary (`Attach.build_id`).
    pub build_id: String,
    pub mode: v1::Mode,
    /// Dev mode (`sortie dev`, FakeDcs): spawns and destroys without `.key()` fail right at the
    /// call site instead of reaching the engine (which rejects them too).
    pub dev: bool,
    /// The `<proc>` part of this logic's command ids, `"<session4>-<proc>-<n>"`. It keeps apart
    /// the ids of two logic processes in one session (a restarted or replaced logic counts from
    /// 1 again). Empty (the default): [`random_proc_token`] when the logic starts. FakeDcs gives
    /// each logic it runs a fixed one.
    pub proc_token: String,
}

impl Default for AttachOptions {
    fn default() -> Self {
        Self {
            logic: "logic".into(),
            build_id: "dev".into(),
            mode: v1::Mode::Primary,
            dev: false,
            proc_token: String::new(),
        }
    }
}

/// Length of a drawn [`AttachOptions::proc_token`].
pub const PROC_TOKEN_LEN: usize = 6;

/// A random command-id token: [`PROC_TOKEN_LEN`] lowercase base36 characters (about 2·10⁹
/// values), drawn once per logic process.
#[must_use]
pub fn random_proc_token() -> String {
    use std::hash::BuildHasher as _;
    // `RandomState` is seeded from the system's randomness; the pid and the clock are mixed in
    // so that two processes started at once still differ.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let n = std::collections::hash_map::RandomState::new().hash_one((std::process::id(), nanos));
    base36(n, PROC_TOKEN_LEN)
}

/// The last `len` base36 digits of `n` (lowercase, zero-padded).
pub(crate) fn base36(mut n: u64, len: usize) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = vec![b'0'; len];
    for c in out.iter_mut().rev() {
        *c = DIGITS[usize::try_from(n % 36).unwrap_or(0)];
        n /= 36;
    }
    out.into_iter().map(char::from).collect()
}

/// The `Attach` message for scenario `M`.
#[must_use]
pub fn attach_message<M: Scenario>(opts: &AttachOptions) -> v1::Attach {
    v1::Attach {
        logic: opts.logic.clone(),
        build_id: opts.build_id.clone(),
        scenario_schema_sha256: sortie_types::schema_sha256_bytes::<M>()
            .map(|b| b.to_vec())
            .unwrap_or_default(),
        symbols_used: M::SYMBOLS.iter().map(sortie_types::Symbol::key).collect(),
        mode: opts.mode as i32,
        scenario: M::NAME.to_owned(),
        schema: M::SCHEMA,
    }
}

// ----- session end ----------------------------------------------------------------------------

/// Why a session ended, from the logic's point of view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EndReason {
    /// The logic called `ctx.end_session(outcome)`.
    Logic,
    /// The manifest's time limit expired.
    TimeLimit,
    /// The mission was restarted (`Missions.Restart`, a new mission load).
    Restart,
    /// DCS unloaded the mission without an outcome.
    MissionUnloaded,
    /// The engine is shutting down.
    EngineShutdown,
    /// The engine started a new session without ending this one first.
    Replaced,
    /// The connection to the engine was lost (the runtime reconnects).
    Disconnected,
    /// The engine did not say.
    Unspecified,
}

impl EndReason {
    fn from_proto(r: v1::EndReason) -> Self {
        match r {
            v1::EndReason::Logic => EndReason::Logic,
            v1::EndReason::TimeLimit => EndReason::TimeLimit,
            v1::EndReason::Restart => EndReason::Restart,
            v1::EndReason::MissionUnloaded => EndReason::MissionUnloaded,
            v1::EndReason::EngineShutdown => EndReason::EngineShutdown,
            v1::EndReason::Unspecified => EndReason::Unspecified,
        }
    }
}

/// How a session ended: what [`Ctx::until_session_end`](crate::Ctx::until_session_end)
/// resolves to.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionEnd {
    pub outcome: Option<Outcome>,
    pub text: String,
    pub reason: EndReason,
}

impl SessionEnd {
    fn without_outcome(reason: EndReason) -> Self {
        Self {
            outcome: None,
            text: String::new(),
            reason,
        }
    }
}

/// Why a pending command or state op will never get its answer.
#[derive(Clone, Debug)]
pub(crate) enum Stop {
    /// The session ended (→ `Error::SessionEnded`).
    Ended(String),
    /// The engine connection was lost (→ `Error::Transport`).
    Transport(String),
}

impl Stop {
    pub(crate) fn into_error(self, failure: Failure) -> Error {
        match self {
            Stop::Ended(message) => Error::SessionEnded(Box::new(Failure { message, ..failure })),
            Stop::Transport(message) => Error::Transport(message),
        }
    }
}

// ----- session internals --------------------------------------------------------------------

/// A task of a session, tracked for error reporting and leak checks.
#[derive(Debug)]
struct TrackedTask {
    what: String,
    handle: AbortHandle,
}

/// State shared by every handle of one session (`Ctx`, `Group`, `MenuCommand`, ...). Opaque
/// outside the SDK (it appears in [`crate::cmd::CommandBody`] signatures).
pub struct Session {
    pub(crate) id: String,
    pub(crate) turn: String,
    pub(crate) logic: String,
    pub(crate) build_id: String,
    pub(crate) scenario: String,
    pub(crate) manifest: Rc<Manifest>,
    pub(crate) dev: bool,
    pub(crate) world: RefCell<World>,
    pub(crate) shared: Rc<LogicShared>,
    up: mpsc::UnboundedSender<v1::LogicUp>,
    pending: RefCell<HashMap<String, oneshot::Sender<Result<v1::CommandResult, Stop>>>>,
    pending_state: RefCell<HashMap<u64, oneshot::Sender<Result<v1::StateResult, Stop>>>>,
    detached_state: RefCell<HashMap<u64, &'static str>>,
    /// Model time, bumped on every tick (wakes `sleep` and timers).
    pub(crate) clock: watch::Sender<f64>,
    /// Bumped on every tick and every event (wakes `wait_until`).
    pub(crate) changes: watch::Sender<u64>,
    paused: Cell<bool>,
    /// A resync was requested (the replica missed a delta); deltas are skipped until the
    /// engine's snapshot arrives.
    resyncing: Cell<bool>,
    pub(crate) events: broadcast::Sender<v1::Event>,
    menus: RefCell<Vec<Weak<MenuInbox>>>,
    pub(crate) end: watch::Sender<Option<SessionEnd>>,
    tasks: RefCell<Vec<TrackedTask>>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("id", &self.id)
            .field("turn", &self.turn)
            .finish_non_exhaustive()
    }
}

impl Session {
    fn new(
        start: &v1::Start,
        opts: &AttachOptions,
        up: &mpsc::UnboundedSender<v1::LogicUp>,
        shared: &Rc<LogicShared>,
    ) -> Self {
        let manifest = start.manifest.clone().unwrap_or_default();
        let mut world = match &start.snapshot {
            Some(snap) => World::from_snapshot(snap),
            None => World::replica(start.session.clone()),
        };
        world.set_zones(&manifest.zones);
        world.set_model_time(start.model_time);
        Session {
            id: start.session.clone(),
            turn: start.turn.clone(),
            logic: opts.logic.clone(),
            build_id: opts.build_id.clone(),
            scenario: manifest.scenario.clone(),
            manifest: Rc::new(manifest),
            dev: opts.dev,
            world: RefCell::new(world),
            shared: shared.clone(),
            up: up.clone(),
            pending: RefCell::default(),
            pending_state: RefCell::default(),
            detached_state: RefCell::default(),
            clock: watch::Sender::new(start.model_time),
            changes: watch::Sender::new(0),
            paused: Cell::new(false),
            resyncing: Cell::new(false),
            events: broadcast::Sender::new(1024),
            menus: RefCell::default(),
            end: watch::Sender::new(None),
            tasks: RefCell::default(),
        }
    }

    /// Command ids are `"<session4>-<proc>-<n>"`: the session id's last 4 characters, this
    /// logic process's token ([`AttachOptions::proc_token`]) and its counter. They are unique
    /// across processes, so an `.after(..)` never names another process's command.
    pub(crate) fn next_command_id(&self) -> String {
        let n = self.shared.next_command.get() + 1;
        self.shared.next_command.set(n);
        let tail = self.id.len().saturating_sub(4);
        format!(
            "{}-{}-{n}",
            self.id.get(tail..).unwrap_or(&self.id),
            self.shared.proc_token
        )
    }

    fn next_state_id(&self) -> u64 {
        let n = self.shared.next_state.get() + 1;
        self.shared.next_state.set(n);
        n
    }

    pub(crate) fn model_time(&self) -> f64 {
        *self.clock.borrow()
    }

    pub(crate) fn paused(&self) -> bool {
        self.paused.get()
    }

    pub(crate) fn is_ended(&self) -> bool {
        self.end.borrow().is_some()
    }

    /// Sends a command and waits for its result.
    pub(crate) async fn submit(&self, mut cmd: v1::Command) -> Result<v1::CommandResult, Stop> {
        if self.is_ended() {
            return Err(Stop::Ended(format!("session {} has ended", self.id)));
        }
        cmd.session.clone_from(&self.id);
        let (tx, rx) = oneshot::channel();
        self.pending.borrow_mut().insert(cmd.id.clone(), tx);
        let msg = v1::LogicUp {
            msg: Some(logic_up::Msg::Command(cmd)),
        };
        if self.up.send(msg).is_err() {
            return Err(Stop::Transport("the engine stream is closed".into()));
        }
        rx.await
            .unwrap_or_else(|_| Err(Stop::Ended("the command was dropped".into())))
    }

    /// Sends a state op and waits for its result.
    pub(crate) async fn state_op(&self, op: v1::state_op::Op) -> Result<v1::StateResult> {
        let failure = || Failure::new("state", "").with_target(format!("session {}", self.id));
        if self.is_ended() {
            return Err(Stop::Ended(format!("session {} has ended", self.id)).into_error(failure()));
        }
        let id = self.next_state_id();
        let (tx, rx) = oneshot::channel();
        self.pending_state.borrow_mut().insert(id, tx);
        let msg = v1::LogicUp {
            msg: Some(logic_up::Msg::State(v1::StateOp { id, op: Some(op) })),
        };
        if self.up.send(msg).is_err() {
            return Err(Error::Transport("the engine stream is closed".into()));
        }
        match rx.await {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(stop)) => Err(stop.into_error(failure())),
            Err(_) => Err(Stop::Ended("the state op was dropped".into()).into_error(failure())),
        }
    }

    /// Sends a state op whose failure is only recorded (see [`LogicHandle::errors`]).
    pub(crate) fn state_op_detached(&self, what: &'static str, op: v1::state_op::Op) {
        if self.is_ended() {
            tracing::debug!(what, "state op after the session ended: dropped");
            return;
        }
        let id = self.next_state_id();
        self.detached_state.borrow_mut().insert(id, what);
        let msg = v1::LogicUp {
            msg: Some(logic_up::Msg::State(v1::StateOp { id, op: Some(op) })),
        };
        if self.up.send(msg).is_err() {
            self.shared
                .record_error(format!("{what}: the engine stream is closed"));
        }
    }

    /// Spawns a tracked task of this session: an `Err` (other than `SessionEnded`) or a panic
    /// is recorded with `what` (which names the call site); the task is aborted at session end.
    pub(crate) fn spawn_task(
        &self,
        what: String,
        fut: impl Future<Output = Result<()>> + 'static,
    ) -> AbortHandle {
        let shared = self.shared.clone();
        let label = what.clone();
        let task = tokio::task::spawn_local(async move {
            match AssertUnwindSafe(fut).catch_unwind().await {
                Ok(Ok(())) => {}
                Ok(Err(e)) if e.is_session_ended() => {}
                Ok(Err(e)) => shared.record_error(format!("{label} returned\n{}", e.report())),
                Err(payload) => {
                    let at = take_panic_location()
                        .map(|l| format!(" at {l}"))
                        .unwrap_or_default();
                    shared.record_error(format!(
                        "{label} panicked{at}: {}",
                        panic_message(payload.as_ref())
                    ));
                }
            }
        });
        let handle = task.abort_handle();
        let tracked = TrackedTask {
            what,
            handle: handle.clone(),
        };
        if self.is_ended() {
            handle.abort();
            self.shared.retire(vec![tracked]);
        } else {
            let mut tasks = self.tasks.borrow_mut();
            tasks.retain(|t| !t.handle.is_finished());
            tasks.push(tracked);
        }
        handle
    }

    pub(crate) fn register_menu(&self, inbox: &Rc<MenuInbox>) {
        let mut menus = self.menus.borrow_mut();
        menus.retain(|m| m.strong_count() > 0);
        menus.push(Rc::downgrade(inbox));
    }

    fn on_tick(&self, tick: &v1::Tick) {
        match self.world.try_borrow_mut() {
            Ok(mut world) => {
                if let Some(delta) = &tick.delta
                    && !self.resyncing.get()
                    && let Err(e) = world.apply_delta(delta)
                {
                    // Recoverable: the engine answers with a full snapshot (`on_resync`).
                    tracing::warn!(tick = tick.n, error = %e, "replica out of sync; asking the engine for a snapshot");
                    self.resyncing.set(true);
                    let _ = self.up.send(v1::LogicUp {
                        msg: Some(logic_up::Msg::Resync(v1::Resync {
                            have_rev: world.rev(),
                            reason: e.to_string(),
                        })),
                    });
                }
                world.set_model_time(tick.model_time);
            }
            Err(_) => self.shared.record_error(format!(
                "tick {}: the world replica is borrowed across an `.await` (a `ctx.world()` \
                 guard is held while awaiting); the tick could not be applied",
                tick.n
            )),
        }
        self.paused.set(tick.paused);
        self.clock.send_if_modified(|t| {
            let changed = *t != tick.model_time;
            *t = tick.model_time;
            changed
        });
        self.changes.send_modify(|n| *n += 1);
    }

    /// `LogicDown.resync`: replaces the replica with the engine's snapshot.
    fn on_resync(&self, snap: &v1::WorldSnapshot) {
        match self.world.try_borrow_mut() {
            Ok(mut world) => {
                world.apply_snapshot(snap);
                self.resyncing.set(false);
            }
            Err(_) => {
                // Still out of sync: ask again on the next tick.
                self.resyncing.set(false);
                self.shared.record_error(
                    "a world resync arrived while the replica was borrowed across an `.await`; \
                     it was not applied",
                );
                return;
            }
        }
        self.changes.send_modify(|n| *n += 1);
    }

    fn on_event(&self, ev: v1::Event) {
        use v1::event::Kind;
        match self.world.try_borrow_mut() {
            Ok(mut world) => world.apply_event(&ev),
            Err(_) => self.shared.record_error(
                "an event arrived while the world replica was borrowed across an `.await`; \
                 it was not applied",
            ),
        }
        match &ev.kind {
            Some(Kind::MenuPressed(p)) => {
                for inbox in self.menus.borrow().iter().filter_map(Weak::upgrade) {
                    inbox.offer(p, ev.model_time);
                }
            }
            Some(Kind::SimulationPaused(_)) => self.paused.set(true),
            Some(Kind::SimulationResumed(_)) => self.paused.set(false),
            _ => {}
        }
        let _ = self.events.send(ev);
        self.changes.send_modify(|n| *n += 1);
    }

    fn deliver_result(&self, r: v1::CommandResult) {
        if let Some(tx) = self.pending.borrow_mut().remove(&r.id) {
            let _ = tx.send(Ok(r));
        } else {
            tracing::debug!(id = %r.id, "result for an unknown command");
        }
    }

    fn deliver_state(&self, r: v1::StateResult) {
        if let Some(tx) = self.pending_state.borrow_mut().remove(&r.id) {
            let _ = tx.send(Ok(r));
        } else if let Some(what) = self.detached_state.borrow_mut().remove(&r.id)
            && let Some(v1::state_result::Result::Err(e)) = r.result
        {
            self.shared.record_error(format!("{what}: {}", e.message));
        }
    }

    /// Ends the session: resolves `until_session_end()`, fails what is pending with `stop`,
    /// lets woken tasks run up to their next `.await`, then aborts every task.
    async fn finish(&self, end: SessionEnd, stop: Stop) {
        tracing::info!(session = %self.id, reason = ?end.reason, outcome = ?end.outcome, "session ended");
        self.end.send_replace(Some(end));
        let pending: Vec<_> = self.pending.borrow_mut().drain().collect();
        for (_, tx) in pending {
            let _ = tx.send(Err(stop.clone()));
        }
        let pending: Vec<_> = self.pending_state.borrow_mut().drain().collect();
        for (_, tx) in pending {
            let _ = tx.send(Err(stop.clone()));
        }
        self.detached_state.borrow_mut().clear();
        tokio::task::yield_now().await;
        let tasks: Vec<TrackedTask> = self.tasks.borrow_mut().drain(..).collect();
        for t in &tasks {
            t.handle.abort();
        }
        self.shared.retire(tasks);
        self.menus.borrow_mut().clear();
    }
}

/// Facts about a running logic, shared with its [`LogicHandle`] (and kept across reconnects).
#[derive(Debug, Default)]
pub(crate) struct LogicShared {
    outcome: Cell<Option<Outcome>>,
    ended: RefCell<Option<v1::SessionEnded>>,
    errors: RefCell<Vec<String>>,
    sessions: Cell<u32>,
    finished: Cell<bool>,
    rejected: RefCell<Option<v1::AttachRejected>>,
    /// The `<proc>` of command ids, fixed for the life of the logic (across reconnects).
    proc_token: String,
    next_command: Cell<u64>,
    next_state: Cell<u64>,
    /// Tasks of ended sessions (aborted; must be finished by the time anyone checks).
    retired: RefCell<Vec<TrackedTask>>,
}

impl LogicShared {
    fn new(opts: &AttachOptions) -> Self {
        Self {
            proc_token: if opts.proc_token.is_empty() {
                random_proc_token()
            } else {
                opts.proc_token.clone()
            },
            ..Self::default()
        }
    }

    pub(crate) fn record_error(&self, e: impl Into<String>) {
        let e = e.into();
        tracing::error!("{e}");
        self.errors.borrow_mut().push(e);
    }

    fn retire(&self, tasks: Vec<TrackedTask>) {
        let mut retired = self.retired.borrow_mut();
        retired.retain(|t| !t.handle.is_finished());
        retired.extend(tasks);
    }
}

/// Observes a logic started with [`spawn_logic`] (what `FakeDcs::run` returns).
#[derive(Clone, Debug)]
pub struct LogicHandle {
    shared: Rc<LogicShared>,
}

impl LogicHandle {
    /// Outcome of the last session that ended (from `SessionEnded`).
    #[must_use]
    pub fn outcome(&self) -> Option<Outcome> {
        self.shared.outcome.get()
    }

    /// The last `SessionEnded` received.
    #[must_use]
    pub fn session_ended(&self) -> Option<v1::SessionEnded> {
        self.shared.ended.borrow().clone()
    }

    /// Errors returned by the mission or its tasks, panics, failed detached ops, replica
    /// problems; each names the Rust call site.
    #[must_use]
    pub fn errors(&self) -> Vec<String> {
        self.shared.errors.borrow().clone()
    }

    /// Number of sessions started so far.
    #[must_use]
    pub fn sessions(&self) -> u32 {
        self.shared.sessions.get()
    }

    /// The runtime loop has exited (shutdown, stream closed or attach rejected).
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.shared.finished.get()
    }

    #[must_use]
    pub fn rejected(&self) -> Option<v1::AttachRejected> {
        self.shared.rejected.borrow().clone()
    }

    /// Tasks of sessions that already ended which are still running (should be none: the
    /// runtime aborts them), as their spawn descriptions.
    #[must_use]
    pub fn leaked_tasks(&self) -> Vec<String> {
        self.shared
            .retired
            .borrow()
            .iter()
            .filter(|t| !t.handle.is_finished())
            .map(|t| t.what.clone())
            .collect()
    }

    /// Panics unless the logic ran cleanly: attached, at least one session started, no
    /// errors or panics recorded, and no task of an ended session still running.
    #[track_caller]
    pub fn assert_clean(&self) {
        if let Some(r) = self.rejected() {
            panic!(
                "logic attach was rejected: {} (missing symbols: {:?})",
                r.message, r.missing_symbols
            );
        }
        let errors = self.errors();
        assert!(
            errors.is_empty(),
            "logic reported {} error(s):\n\n{}",
            errors.len(),
            errors.join("\n\n")
        );
        assert!(
            self.sessions() > 0,
            "the logic never received a session Start"
        );
        let leaked = self.leaked_tasks();
        assert!(
            leaked.is_empty(),
            "{} task(s) outlived their session: {}",
            leaked.len(),
            leaked.join(", ")
        );
    }
}

/// Runs a logic over `conn` as a local task of the current `LocalSet` and returns its handle.
///
/// # Panics
/// Outside a `LocalSet` (tokio's `spawn_local` requirement).
pub fn spawn_logic<M: Scenario, F: Mission<M>>(
    conn: LogicConn,
    mission: F,
    opts: AttachOptions,
) -> LogicHandle {
    let shared = Rc::new(LogicShared::new(&opts));
    let handle = LogicHandle {
        shared: shared.clone(),
    };
    tokio::task::spawn_local(async move {
        drive::<M, F>(conn, &Rc::new(mission), &opts, &shared).await;
        shared.finished.set(true);
    });
    handle
}

/// Runs a logic over `conn` until the stream ends; resolves to its handle.
pub async fn run_logic<M: Scenario, F: Mission<M>>(
    conn: LogicConn,
    mission: F,
    opts: AttachOptions,
) -> LogicHandle {
    let shared = Rc::new(LogicShared::new(&opts));
    drive::<M, F>(conn, &Rc::new(mission), &opts, &shared).await;
    shared.finished.set(true);
    LogicHandle { shared }
}

/// How one Logic stream ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Exit {
    Shutdown,
    Rejected,
    Closed,
}

async fn drive<M: Scenario, F: Mission<M>>(
    conn: LogicConn,
    mission: &Rc<F>,
    opts: &AttachOptions,
    shared: &Rc<LogicShared>,
) -> Exit {
    install_panic_hook();
    let LogicConn { up, mut down } = conn;
    let attach = attach_message::<M>(opts);
    if up
        .send(v1::LogicUp {
            msg: Some(logic_up::Msg::Attach(attach)),
        })
        .is_err()
    {
        tracing::warn!("the engine stream closed before Attach");
        return Exit::Closed;
    }
    let mut session: Option<Rc<Session>> = None;
    let exit = loop {
        let Some(msg) = down.recv().await else {
            break Exit::Closed;
        };
        let Some(msg) = msg.msg else { continue };
        match msg {
            logic_down::Msg::Start(start) => {
                if let Some(old) = session.take() {
                    old.finish(
                        SessionEnd::without_outcome(EndReason::Replaced),
                        Stop::Ended("a new session started".into()),
                    )
                    .await;
                }
                let s = Rc::new(Session::new(&start, opts, &up, shared));
                shared.sessions.set(shared.sessions.get() + 1);
                let kind = Start::from_proto(start.kind());
                tracing::info!(session = %s.id, ?kind, scenario = %s.scenario, "session started");
                let run = mission.start(Ctx::<M>::from_session(s.clone()), kind);
                s.spawn_task(
                    format!("mission function `{}`", std::any::type_name::<F>()),
                    run,
                );
                session = Some(s);
            }
            logic_down::Msg::Tick(tick) => {
                if let Some(s) = &session {
                    s.on_tick(&tick);
                    // Let every task woken by this tick run until it blocks again.
                    tokio::task::yield_now().await;
                }
                let _ = up.send(v1::LogicUp {
                    msg: Some(logic_up::Msg::Done(v1::TickDone { n: tick.n })),
                });
            }
            logic_down::Msg::Event(ev) => {
                if let Some(s) = &session {
                    s.on_event(ev);
                }
            }
            logic_down::Msg::Resync(snap) => {
                if let Some(s) = &session
                    && (snap.session.is_empty() || snap.session == s.id)
                {
                    s.on_resync(&snap);
                }
            }
            logic_down::Msg::Result(r) => {
                if let Some(s) = &session {
                    s.deliver_result(r);
                }
            }
            logic_down::Msg::State(r) => {
                if let Some(s) = &session {
                    s.deliver_state(r);
                }
            }
            logic_down::Msg::Ended(ended) => {
                if let Some(s) = &session
                    && !ended.session.is_empty()
                    && ended.session != s.id
                {
                    tracing::debug!(ended = %ended.session, current = %s.id, "SessionEnded for another session");
                    continue;
                }
                let outcome = Outcome::from_i32(ended.outcome);
                shared.outcome.set(outcome);
                let end = SessionEnd {
                    outcome,
                    text: ended.text.clone(),
                    reason: EndReason::from_proto(ended.reason()),
                };
                *shared.ended.borrow_mut() = Some(ended);
                if let Some(s) = session.take() {
                    s.finish(end, Stop::Ended(format!("session {} has ended", s.id)))
                        .await;
                }
            }
            logic_down::Msg::Promote(_) => {
                tracing::info!("promoted to primary (hot reload lands in M4)");
            }
            logic_down::Msg::Shutdown(s) => {
                tracing::info!(reason = %s.reason, "the engine shut this logic down");
                if let Some(s) = session.take() {
                    s.finish(
                        SessionEnd::without_outcome(EndReason::EngineShutdown),
                        Stop::Ended("the engine shut the logic down".into()),
                    )
                    .await;
                }
                break Exit::Shutdown;
            }
            logic_down::Msg::Rejected(r) => {
                shared.record_error(format!(
                    "attach rejected: {}{}",
                    r.message,
                    if r.missing_symbols.is_empty() {
                        String::new()
                    } else {
                        format!(" (missing symbols: {})", r.missing_symbols.join(", "))
                    }
                ));
                *shared.rejected.borrow_mut() = Some(r);
                break Exit::Rejected;
            }
        }
    };
    if let Some(s) = session.take() {
        s.finish(
            SessionEnd::without_outcome(EndReason::Disconnected),
            Stop::Transport("the engine connection was lost".into()),
        )
        .await;
    }
    exit
}

// ----- panics -------------------------------------------------------------------------------

thread_local! {
    static LAST_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Installs (once per process) a panic hook that remembers where the last panic on this thread
/// happened, then defers to the previous hook (so panics still print as usual).
fn install_panic_hook() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if let Some(l) = info.location() {
                let at = format!("{}:{}:{}", l.file(), l.line(), l.column());
                let _ = LAST_PANIC.try_with(|p| *p.borrow_mut() = Some(at));
            }
            previous(info);
        }));
    });
}

fn take_panic_location() -> Option<String> {
    LAST_PANIC
        .try_with(|p| p.borrow_mut().take())
        .ok()
        .flatten()
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "(non-string panic payload)".to_owned()
    }
}

// ----- the logic binary's main ---------------------------------------------------------------

/// Default engine gRPC endpoint (`sortied --grpc`).
pub const DEFAULT_ENGINE: &str = "http://127.0.0.1:50777";

/// First and longest delay between reconnect attempts.
pub const RECONNECT_BACKOFF: (Duration, Duration) =
    (Duration::from_secs(1), Duration::from_secs(30));

/// Command-line / environment options of a logic binary.
///
/// * `--engine <url>` or `SORTIE_ENGINE` (default [`DEFAULT_ENGINE`]; `host:port` is accepted);
/// * `SORTIE_TOKEN`: bearer token sent as `authorization: Bearer <token>` (role `logic`);
/// * `--build-id <id>` or `SORTIE_BUILD_ID` (default: `<crate>@<version>`);
/// * `--logic <name>` (default: the binary name);
/// * `--standby` or `SORTIE_MODE=standby` (hot reload, M4);
/// * `--dev` or `SORTIE_DEV=1` (set by `sortie dev`: spawns/destroys need `.key()`).
#[derive(Clone, Debug)]
pub struct Options {
    pub engine: String,
    pub token: Option<String>,
    pub attach: AttachOptions,
    /// `--help` was given.
    pub help: bool,
}

/// What `--help` prints (`<logic>` is replaced with the binary name, see [`usage`]).
pub const USAGE: &str = "\
usage: <logic> [--engine <url>] [--build-id <id>] [--logic <name>] [--standby] [--dev]

  --engine <url>    engine gRPC endpoint (env SORTIE_ENGINE, default http://127.0.0.1:50777)
  --build-id <id>   build id announced to the engine (env SORTIE_BUILD_ID)
  --logic <name>    logic name announced to the engine (default: the binary name)
  --standby         attach as a hot-reload standby (env SORTIE_MODE=standby; lands in M4)
  --dev             dev mode: spawns and destroys must have .key() (env SORTIE_DEV=1)

  The bearer token is read from SORTIE_TOKEN. Log filter: SORTIE_LOG (default info).";

/// [`USAGE`] for the binary `logic`.
#[must_use]
pub fn usage(logic: &str) -> String {
    USAGE.replacen("<logic>", logic, 1)
}

impl Options {
    /// Parses `args` (without the program name) over the environment. `logic` and `version`
    /// are the defaults for the logic name and the build id.
    pub fn parse(
        args: impl IntoIterator<Item = String>,
        env: impl Fn(&str) -> Option<String>,
        logic: &str,
        version: &str,
    ) -> Result<Self, String> {
        let truthy =
            |v: Option<String>| v.is_some_and(|v| !matches!(v.as_str(), "" | "0" | "false"));
        let mut o = Options {
            engine: env("SORTIE_ENGINE").unwrap_or_else(|| DEFAULT_ENGINE.to_owned()),
            token: env("SORTIE_TOKEN").filter(|t| !t.is_empty()),
            attach: AttachOptions {
                logic: logic.to_owned(),
                build_id: env("SORTIE_BUILD_ID").unwrap_or_else(|| format!("{logic}@{version}")),
                mode: if env("SORTIE_MODE").as_deref() == Some("standby") {
                    v1::Mode::Standby
                } else {
                    v1::Mode::Primary
                },
                dev: truthy(env("SORTIE_DEV")),
                proc_token: String::new(),
            },
            help: false,
        };
        let mut args = args.into_iter();
        while let Some(a) = args.next() {
            match a.as_str() {
                "--engine" => o.engine = args.next().ok_or("--engine needs a URL")?,
                "--build-id" => {
                    o.attach.build_id = args.next().ok_or("--build-id needs a value")?;
                }
                "--logic" => o.attach.logic = args.next().ok_or("--logic needs a name")?,
                "--standby" => o.attach.mode = v1::Mode::Standby,
                "--dev" => o.attach.dev = true,
                "-h" | "--help" => o.help = true,
                other => {
                    return Err(format!(
                        "unknown argument `{other}` (expected --engine, --build-id, --logic, \
                         --standby, --dev or --help)"
                    ));
                }
            }
        }
        if !o.engine.contains("://") {
            o.engine = format!("http://{}", o.engine);
        }
        Ok(o)
    }
}

/// Entry point used by [`crate::main!`] (the logic name and version come from the binary's
/// crate): parses [`Options`], connects to the engine's `Logic.Attach` and runs sessions until
/// the engine shuts the logic down, reconnecting with backoff when the connection drops.
///
/// Exit codes: 0 after `Shutdown`, 2 for bad arguments or a rejected attach (rebuild the
/// logic against the current scenario lock), 1 if the runtime could not start.
pub fn main_named<M: Scenario, F: Mission<M>>(mission: F, logic: &str, version: &str) -> ExitCode {
    init_tracing();
    let opts = match Options::parse(
        std::env::args().skip(1),
        |k| std::env::var(k).ok(),
        logic,
        version,
    ) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}\n\n{}", usage(logic));
            return ExitCode::from(2);
        }
    };
    if opts.help {
        println!("{}", usage(logic));
        return ExitCode::SUCCESS;
    }
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: cannot start the tokio runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, serve::<M, F>(mission, opts))
}

/// [`main_named`] with the logic name taken from the executable's file name.
pub fn main<M: Scenario, F: Mission<M>>(mission: F) -> ExitCode {
    let exe = std::env::args().next().unwrap_or_default();
    let name = std::path::Path::new(&exe)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("logic")
        .to_owned();
    main_named::<M, F>(mission, &name, "0.0.0")
}

/// The reconnecting attach loop behind [`main_named`] (for programs that build their own
/// runtime; must run inside a `LocalSet`).
pub async fn serve<M: Scenario, F: Mission<M>>(mission: F, opts: Options) -> ExitCode {
    let shared = Rc::new(LogicShared::new(&opts.attach));
    let mission = Rc::new(mission);
    let (first, longest) = RECONNECT_BACKOFF;
    let mut backoff = first;
    loop {
        let conn = match open_attach(&opts.engine, opts.token.as_deref()).await {
            Ok(conn) => conn,
            Err(ConnectError::Refused(e)) => {
                eprintln!("error[sortie::transport]: {e}");
                return ExitCode::from(2);
            }
            Err(ConnectError::Unreachable(e)) => {
                tracing::warn!("{e}; retrying in {} s", backoff.as_secs());
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(longest);
                continue;
            }
        };
        tracing::info!(engine = %opts.engine, logic = %opts.attach.logic, scenario = M::NAME, "attaching");
        let sessions_before = shared.sessions.get();
        match drive::<M, F>(conn, &mission, &opts.attach, &shared).await {
            Exit::Shutdown => return ExitCode::SUCCESS,
            Exit::Rejected => {
                if let Some(r) = shared.rejected.borrow().as_ref() {
                    eprintln!(
                        "error[sortie::rejected]: the engine rejected this logic: {}",
                        r.message
                    );
                    if !r.missing_symbols.is_empty() {
                        eprintln!("  missing symbols: {}", r.missing_symbols.join(", "));
                    }
                    eprintln!(
                        " help: rebuild the logic against the scenario lock of the running \
                         mission (`sortie-gen lock`, then `cargo build`)"
                    );
                }
                return ExitCode::from(2);
            }
            Exit::Closed => {
                if shared.sessions.get() > sessions_before {
                    backoff = first;
                }
                tracing::warn!(
                    "the engine connection was lost; reconnecting in {} s",
                    backoff.as_secs()
                );
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(longest);
            }
        }
    }
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_env("SORTIE_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

/// Why [`connect`] failed: the engine is not there (retry), or it refused the credentials
/// (retrying with the same token cannot help).
#[derive(Debug)]
enum ConnectError {
    Unreachable(String),
    Refused(String),
}

impl From<ConnectError> for Error {
    fn from(e: ConnectError) -> Self {
        match e {
            ConnectError::Unreachable(m) | ConnectError::Refused(m) => Error::Transport(m),
        }
    }
}

/// Opens `Logic.Attach` on the engine at `endpoint` (with `authorization: Bearer <token>` when
/// a token is given) and adapts it to a [`LogicConn`]. Fails with [`Error::Transport`] when the
/// engine cannot be reached or refuses the token; a logic the engine rejects later (schema
/// mismatch) gets `AttachRejected` on the stream.
///
/// Must run inside a `LocalSet` (the stream is pumped by a local task).
pub async fn connect(endpoint: &str, token: Option<&str>) -> Result<LogicConn> {
    Ok(open_attach(endpoint, token).await?)
}

async fn open_attach(
    endpoint: &str,
    token: Option<&str>,
) -> std::result::Result<LogicConn, ConnectError> {
    use sortie_proto::v1::logic_client::LogicClient;
    use tonic::metadata::MetadataValue;
    let channel = tonic::transport::Endpoint::from_shared(endpoint.to_owned())
        .map_err(|e| ConnectError::Refused(format!("invalid engine address `{endpoint}`: {e}")))?
        .connect()
        .await
        .map_err(|e| {
            ConnectError::Unreachable(format!(
                "cannot reach the engine at {endpoint}: {} (is `sortied` running?)",
                error_chain(&e)
            ))
        })?;
    let (up_tx, up_rx) = mpsc::unbounded_channel();
    let mut request =
        tonic::Request::new(tokio_stream::wrappers::UnboundedReceiverStream::new(up_rx));
    if let Some(token) = token {
        let value = MetadataValue::try_from(format!("Bearer {token}")).map_err(|_| {
            ConnectError::Refused("SORTIE_TOKEN is not a valid header value".into())
        })?;
        request.metadata_mut().insert("authorization", value);
    }
    // `Start` and `resync` carry the whole world: above tonic's 4 MiB default.
    let mut client = LogicClient::new(channel)
        .max_decoding_message_size(sortie_proto::LOGIC_MAX_MESSAGE)
        .max_encoding_message_size(sortie_proto::LOGIC_MAX_MESSAGE);
    let mut stream = match client.attach(request).await {
        Ok(response) => response.into_inner(),
        Err(status) => {
            let text = format!(
                "Logic.Attach at {endpoint} failed: {}",
                status_text(&status)
            );
            return Err(match status.code() {
                tonic::Code::Unauthenticated | tonic::Code::PermissionDenied => {
                    ConnectError::Refused(text)
                }
                _ => ConnectError::Unreachable(text),
            });
        }
    };
    let (down_tx, down_rx) = mpsc::channel(DOWN_BUFFER);
    tokio::task::spawn_local(async move {
        // Keep the channel (and its connection) alive while the stream is read.
        let _client = client;
        loop {
            match stream.message().await {
                Ok(Some(msg)) => {
                    if down_tx.send(msg).await.is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(status) => {
                    tracing::warn!("Logic.Attach stream broke: {}", status_text(&status));
                    break;
                }
            }
        }
    });
    Ok(LogicConn {
        up: up_tx,
        down: down_rx,
    })
}

/// `error: cause: cause...` (tonic's transport errors say little without their sources).
fn error_chain(e: &dyn std::error::Error) -> String {
    let mut text = e.to_string();
    let mut source = e.source();
    while let Some(cause) = source {
        let s = cause.to_string();
        if !text.ends_with(&s) {
            text.push_str(": ");
            text.push_str(&s);
        }
        source = cause.source();
    }
    text
}

fn status_text(status: &tonic::Status) -> String {
    match status.code() {
        tonic::Code::Unauthenticated | tonic::Code::PermissionDenied => format!(
            "{} (set SORTIE_TOKEN to a token with the `logic` role)",
            status.message()
        ),
        code => format!("{code:?}: {}", status.message()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_parse_args_over_env() {
        let env = |k: &str| match k {
            "SORTIE_ENGINE" => Some("10.0.0.2:50051".to_owned()),
            "SORTIE_TOKEN" => Some("s3cret".to_owned()),
            "SORTIE_DEV" => Some("1".to_owned()),
            _ => None,
        };
        let o = Options::parse(["--standby".to_owned()], env, "op_north", "0.1.0").unwrap();
        assert_eq!(o.engine, "http://10.0.0.2:50051");
        assert_eq!(o.token.as_deref(), Some("s3cret"));
        assert_eq!(o.attach.mode, v1::Mode::Standby);
        assert_eq!(o.attach.build_id, "op_north@0.1.0");
        assert!(o.attach.dev);
        let o = Options::parse(Vec::new(), |_| None, "x", "1").unwrap();
        assert_eq!(o.engine, DEFAULT_ENGINE);
        assert!(!o.attach.dev && o.token.is_none());
        assert!(
            Options::parse(["--bogus".to_owned()], |_| None, "x", "1")
                .unwrap_err()
                .contains("--bogus")
        );
        assert!(
            Options::parse(["--help".to_owned()], |_| None, "x", "1")
                .unwrap()
                .help
        );
    }

    #[test]
    fn proc_tokens_are_short_lowercase_base36() {
        assert_eq!(base36(0, 6), "000000");
        assert_eq!(base36(35, 6), "00000z");
        assert_eq!(base36(36, 6), "000010");
        // Only the last digits are kept.
        assert_eq!(base36(36u64.pow(6) + 1, 6), "000001");
        let a = random_proc_token();
        let b = random_proc_token();
        for t in [&a, &b] {
            assert_eq!(t.len(), PROC_TOKEN_LEN, "{t}");
            assert!(
                t.bytes()
                    .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase()),
                "{t}"
            );
        }
        assert_ne!(a, b, "two draws differ");
    }

    #[test]
    fn panic_messages_are_extracted() {
        let p: Box<dyn std::any::Any + Send> = Box::new("boom");
        assert_eq!(panic_message(p.as_ref()), "boom");
        let p: Box<dyn std::any::Any + Send> = Box::new(String::from("bang"));
        assert_eq!(panic_message(p.as_ref()), "bang");
    }
}
