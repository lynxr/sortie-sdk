//! [`Ctx<M>`]: the session context.

use std::cell::Ref;
use std::marker::PhantomData;
use std::panic::Location;
use std::rc::Rc;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use sortie_proto::manifest::v1::Manifest;
use sortie_proto::v1;
use sortie_types::{
    Coalition, FlagRef, GroupRef, Kind, MapPoint, Outcome, Scenario, SmokeColor, SymbolKind,
    TemplateSpawn, Vec3,
};
use sortie_world::World;

use crate::cmd::{self, Cmd};
use crate::events::Events;
use crate::group::{AnyGroup, Group};
use crate::menu::{CoalitionWide, MenuBuilder};
use crate::runtime::{Session, SessionEnd};
use crate::state::StateDoc;
use crate::{Error, Result};

/// Why the mission function was started.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Start {
    /// First start of this session.
    Fresh,
    /// The mission was restarted in DCS (new session, same turn).
    Restart,
    /// The logic process crashed and was restarted inside the same session.
    AfterCrash,
    /// The engine restarted while DCS kept running and resumed the session under its id (keys,
    /// `once` markers, timers and session documents carry on).
    Resumed,
}

impl Start {
    #[must_use]
    pub fn is_fresh(self) -> bool {
        self == Start::Fresh
    }

    #[must_use]
    pub fn from_proto(k: v1::StartKind) -> Self {
        match k {
            v1::StartKind::Restart => Start::Restart,
            v1::StartKind::AfterCrash => Start::AfterCrash,
            v1::StartKind::Resumed => Start::Resumed,
            v1::StartKind::Fresh | v1::StartKind::Unspecified => Start::Fresh,
        }
    }
}

/// The context of one session, typed by the scenario marker `M` (a handle of another scenario
/// does not compile). Cheap to clone; not `Send` (the runtime is single-threaded).
pub struct Ctx<M> {
    pub(crate) session: Rc<Session>,
    _m: PhantomData<fn() -> M>,
}

impl<M> Clone for Ctx<M> {
    fn clone(&self) -> Self {
        Self {
            session: self.session.clone(),
            _m: PhantomData,
        }
    }
}

impl<M> std::fmt::Debug for Ctx<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ctx")
            .field("session", &self.session.id)
            .finish()
    }
}

/// Handle of a task started with [`Ctx::spawn`]; dropping it does not stop the task (tasks
/// end with their session).
#[derive(Debug)]
pub struct TaskHandle {
    handle: tokio::task::AbortHandle,
}

impl TaskHandle {
    pub fn abort(&self) {
        self.handle.abort();
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}

/// Where a message goes.
#[derive(Clone, Debug, PartialEq)]
pub enum MessageTarget {
    All,
    Coalition(Coalition),
    /// One group (`trigger.action.outTextForGroup`); `name` is for error messages.
    Group {
        sel: v1::GroupSel,
        name: String,
    },
}

impl From<Coalition> for MessageTarget {
    fn from(c: Coalition) -> Self {
        MessageTarget::Coalition(c)
    }
}

impl From<AnyGroup> for MessageTarget {
    fn from(g: AnyGroup) -> Self {
        MessageTarget::Group {
            sel: g.selector(),
            name: g.name,
        }
    }
}

impl From<&AnyGroup> for MessageTarget {
    fn from(g: &AnyGroup) -> Self {
        MessageTarget::Group {
            sel: g.selector(),
            name: g.name.clone(),
        }
    }
}

impl<K> From<&Group<K>> for MessageTarget {
    fn from(g: &Group<K>) -> Self {
        MessageTarget::Group {
            sel: g.selector(),
            name: g.name().to_owned(),
        }
    }
}

/// A point for effects: on the ground (`MapPoint`) or in space (`Vec3`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Position {
    Ground(MapPoint),
    At(Vec3),
}

impl From<MapPoint> for Position {
    fn from(p: MapPoint) -> Self {
        Position::Ground(p)
    }
}

impl From<Vec3> for Position {
    fn from(p: Vec3) -> Self {
        Position::At(p)
    }
}

/// Value of a mission flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlagValue(pub i32);

impl From<bool> for FlagValue {
    fn from(b: bool) -> Self {
        FlagValue(i32::from(b))
    }
}

impl From<i32> for FlagValue {
    fn from(v: i32) -> Self {
        FlagValue(v)
    }
}

/// A mission flag of scenario `M` (see [`Ctx::flag`]).
#[derive(Debug)]
pub struct FlagHandle<M> {
    session: Rc<Session>,
    flag: FlagRef<M>,
}

impl<M> FlagHandle<M> {
    #[track_caller]
    pub fn set(&self, value: impl Into<FlagValue>) -> Cmd<cmd::SetFlag> {
        let body = cmd::SetFlag {
            ident: self.flag.ident(),
            value: value.into().0,
        };
        Cmd::new(self.session.clone(), body, Location::caller())
    }

    #[track_caller]
    pub fn get(&self) -> Cmd<cmd::GetFlag> {
        Cmd::new(
            self.session.clone(),
            cmd::GetFlag {
                ident: self.flag.ident(),
            },
            Location::caller(),
        )
    }
}

/// The session report (`MissionResults.report_json`), read by the generator's worker.
#[derive(Debug)]
pub struct Report {
    session: Rc<Session>,
}

impl Report {
    /// Sets `key` to `value` (JSON). Fire-and-forget: a failure is recorded in the logic's
    /// errors (and fails `LogicHandle::assert_clean`).
    pub fn set(&self, key: &str, value: impl Serialize) {
        match serde_json::to_string(&value) {
            Ok(json) => self.session.state_op_detached(
                "report.set",
                v1::state_op::Op::ReportSet(v1::ReportSet {
                    key: key.to_owned(),
                    value_json: json,
                }),
            ),
            Err(e) => self
                .session
                .shared
                .record_error(format!("report.set({key}): {e}")),
        }
    }
}

impl<M: Scenario> Ctx<M> {
    pub(crate) fn from_session(session: Rc<Session>) -> Self {
        Self {
            session,
            _m: PhantomData,
        }
    }

    // ----- session facts ---------------------------------------------------------------------

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session.id
    }

    /// Scenario name from the manifest (equals `M::NAME` once attached).
    #[must_use]
    pub fn scenario(&self) -> &str {
        &self.session.scenario
    }

    /// Campaign turn key (the running mission's build id).
    #[must_use]
    pub fn turn(&self) -> &str {
        &self.session.turn
    }

    #[must_use]
    pub fn manifest(&self) -> &Manifest {
        &self.session.manifest
    }

    /// Current model time, seconds.
    #[must_use]
    pub fn model_time(&self) -> f64 {
        self.session.model_time()
    }

    /// The local world replica. Do not hold the borrow across `.await` (the runtime could not
    /// apply the next tick; clippy's `await_holding_refcell_ref` catches it).
    #[must_use]
    pub fn world(&self) -> Ref<'_, World> {
        self.session.world.borrow()
    }

    /// Is the simulation paused (as of the last tick or pause event)?
    #[must_use]
    pub fn paused(&self) -> bool {
        self.session.paused()
    }

    /// `true` once the session ended (its tasks are about to be aborted).
    #[must_use]
    pub fn is_session_ended(&self) -> bool {
        self.session.is_ended()
    }

    // ----- reading -----------------------------------------------------------------------------

    /// The live group behind a scenario handle, checked against the manifest:
    /// `Error::UnknownSymbol` if the running mission has no such group (schema drift),
    /// `Error::NotInWorld` if it has one but no live object right now.
    #[track_caller]
    pub fn group<K: Kind>(&self, group: GroupRef<M, K>) -> Result<Group<K>> {
        let at = Location::caller();
        let origin = Some(format!("{}:{}:{}", at.file(), at.line(), at.column()));
        if let Ok(world) = self.session.world.try_borrow()
            && let Some(entry) = world
                .group_entry(sortie_world::GroupKey::Ident(group.ident()))
                .filter(|_| world.group(&group).alive_count() > 0)
        {
            return Ok(Group::new(
                self.session.clone(),
                entry.eid,
                entry.name.clone(),
            ));
        }
        if sortie_manifest::group(&self.session.manifest, group.ident()).is_some() {
            Err(Error::NotInWorld {
                kind: "group",
                ident: group.ident().to_owned(),
                origin,
            })
        } else {
            Err(Error::UnknownSymbol {
                kind: SymbolKind::Group.as_str(),
                ident: group.ident().to_owned(),
                origin,
            })
        }
    }

    /// The events of this session, from now on. Each event is applied to the replica before
    /// you see it.
    #[must_use]
    pub fn events(&self) -> Events {
        Events::new(self.session.clone())
    }

    // ----- waiting -----------------------------------------------------------------------------

    /// Resolves when `pred` holds on the replica: checked now, after every tick and after every
    /// event (so a `Dead` that empties a group wakes it without waiting for the next tick).
    pub async fn wait_until(&self, mut pred: impl FnMut(&World) -> bool) {
        let mut changes = self.session.changes.subscribe();
        loop {
            let hit = self.session.world.try_borrow().is_ok_and(|w| pred(&w));
            if hit {
                return;
            }
            if changes.changed().await.is_err() {
                // The session is gone: the task is about to be aborted.
                std::future::pending::<()>().await;
            }
        }
    }

    /// Resolves when the session ends, with its outcome. Tasks get until their next `.await`
    /// after that to clean up; then every task of the session is aborted. Commands sent after
    /// the end fail with `Error::SessionEnded`.
    pub async fn until_session_end(&self) -> SessionEnd {
        let mut end = self.session.end.subscribe();
        loop {
            let ended = end.borrow_and_update().clone();
            if let Some(e) = ended {
                return e;
            }
            if end.changed().await.is_err() {
                std::future::pending::<()>().await;
            }
        }
    }

    /// Sleeps for `dur` of model time (pauses stop it).
    pub async fn sleep(&self, dur: Duration) {
        let deadline = self.model_time() + dur.as_secs_f64();
        self.sleep_until(deadline).await;
    }

    async fn sleep_until(&self, deadline: f64) {
        let mut clock = self.session.clock.subscribe();
        loop {
            let now = *clock.borrow_and_update();
            if now >= deadline {
                return;
            }
            if clock.changed().await.is_err() {
                std::future::pending::<()>().await;
            }
        }
    }

    /// A named session timer: the first call stores `deadline = now + dur` in the engine, later
    /// calls (after a reload or crash) wait for the stored deadline.
    pub async fn timer(&self, name: &str, dur: Duration) {
        let op = v1::state_op::Op::TimerArm(v1::TimerArm {
            name: name.to_owned(),
            duration_s: dur.as_secs_f64(),
        });
        let deadline = match self.session.state_op(op).await {
            Ok(v1::StateResult {
                result: Some(v1::state_result::Result::Timer(t)),
                ..
            }) => t.deadline,
            Ok(other) => {
                self.session.shared.record_error(format!(
                    "timer `{name}`: unexpected reply {other:?}; using a local deadline"
                ));
                self.model_time() + dur.as_secs_f64()
            }
            Err(e) => {
                self.session
                    .shared
                    .record_error(format!("timer `{name}`: {e}; using a local deadline"));
                self.model_time() + dur.as_secs_f64()
            }
        };
        self.sleep_until(deadline).await;
    }

    /// Runs `f` once per session under `name`.
    pub async fn once<T, F>(&self, name: &str, f: F) -> Result<Option<T>>
    where
        F: AsyncFnOnce(Ctx<M>) -> Result<T>,
    {
        self.once_in(v1::StateScope::Session, name, f).await
    }

    /// Runs `f` once per campaign turn under `name` (survives mission restarts within a turn).
    pub async fn once_per_turn<T, F>(&self, name: &str, f: F) -> Result<Option<T>>
    where
        F: AsyncFnOnce(Ctx<M>) -> Result<T>,
    {
        self.once_in(v1::StateScope::Turn, name, f).await
    }

    async fn once_in<T, F>(&self, scope: v1::StateScope, name: &str, f: F) -> Result<Option<T>>
    where
        F: AsyncFnOnce(Ctx<M>) -> Result<T>,
    {
        let check = v1::state_op::Op::OnceCheck(v1::OnceCheck {
            scope: scope as i32,
            name: name.to_owned(),
        });
        match self.session.state_op(check).await?.result {
            Some(v1::state_result::Result::Once(o)) if o.done => return Ok(None),
            Some(v1::state_result::Result::Once(_)) => {}
            Some(v1::state_result::Result::Err(e)) => {
                return Err(Error::State {
                    name: name.to_owned(),
                    message: e.message,
                });
            }
            other => {
                return Err(Error::State {
                    name: name.to_owned(),
                    message: format!("unexpected reply {other:?}"),
                });
            }
        }
        let value = f(self.clone()).await?;
        let mark = v1::state_op::Op::OnceMark(v1::OnceMark {
            scope: scope as i32,
            name: name.to_owned(),
        });
        if let Some(v1::state_result::Result::Err(e)) = self.session.state_op(mark).await?.result {
            return Err(Error::State {
                name: name.to_owned(),
                message: e.message,
            });
        }
        Ok(Some(value))
    }

    // ----- state -------------------------------------------------------------------------------

    /// The campaign-scoped state document `name` (created from `T::default()` if missing).
    pub async fn state<T>(&self, name: &str) -> Result<StateDoc<T>>
    where
        T: Serialize + DeserializeOwned + Default + 'static,
    {
        StateDoc::load(self.session.clone(), v1::StateScope::Campaign, name).await
    }

    /// A state document in another scope (session, turn).
    pub async fn state_in<T>(&self, scope: v1::StateScope, name: &str) -> Result<StateDoc<T>>
    where
        T: Serialize + DeserializeOwned + Default + 'static,
    {
        StateDoc::load(self.session.clone(), scope, name).await
    }

    #[must_use]
    pub fn report(&self) -> Report {
        Report {
            session: self.session.clone(),
        }
    }

    // ----- tasks -------------------------------------------------------------------------------

    /// Starts a task that lives as long as the session (it is aborted when the session ends).
    /// An `Err` it returns (other than `SessionEnded`) or a panic is recorded as a logic error
    /// with this call site, and fails `LogicHandle::assert_clean`; other tasks keep running.
    #[track_caller]
    pub fn spawn<F>(&self, f: F) -> TaskHandle
    where
        F: AsyncFnOnce(Ctx<M>) -> Result<()> + 'static,
    {
        let at = Location::caller();
        let ctx = self.clone();
        let handle = self.session.spawn_task(
            format!(
                "task spawned at {}:{}:{}",
                at.file(),
                at.line(),
                at.column()
            ),
            async move { f(ctx).await },
        );
        TaskHandle { handle }
    }

    // ----- writing -----------------------------------------------------------------------------

    #[track_caller]
    pub fn message(&self, to: impl Into<MessageTarget>) -> Cmd<cmd::SendMessage> {
        Cmd::new(
            self.session.clone(),
            cmd::SendMessage::new(to.into()),
            Location::caller(),
        )
    }

    /// A radio (F10) menu for `coalition`; `.per_group()` installs it per group.
    #[must_use]
    pub fn menu(&self, coalition: Coalition) -> MenuBuilder<M, CoalitionWide> {
        MenuBuilder::new(self.session.clone(), coalition)
    }

    #[track_caller]
    pub fn smoke(&self, at: impl Into<Position>, color: SmokeColor) -> Cmd<cmd::Smoke> {
        Cmd::new(
            self.session.clone(),
            cmd::Smoke {
                at: at.into(),
                color,
            },
            Location::caller(),
        )
    }

    #[must_use]
    pub fn flag(&self, flag: FlagRef<M>) -> FlagHandle<M> {
        FlagHandle {
            session: self.session.clone(),
            flag,
        }
    }

    /// Spawns a group from a template: `.from_airbase(...)` or `.at(...)`, then `.await`.
    #[track_caller]
    pub fn spawn_group<K: Kind>(
        &self,
        template: impl Into<TemplateSpawn<M, K>>,
    ) -> Cmd<cmd::SpawnGroup<K>> {
        let t = template.into();
        Cmd::new(
            self.session.clone(),
            cmd::SpawnGroup::new(t.template.ident(), t.name.map(Into::into)),
            Location::caller(),
        )
    }

    /// Ends the session (the turn) with `outcome`.
    #[track_caller]
    pub fn end_session(&self, outcome: Outcome) -> Cmd<cmd::EndSession> {
        Cmd::new(
            self.session.clone(),
            cmd::EndSession {
                outcome,
                text: String::new(),
            },
            Location::caller(),
        )
    }
}
