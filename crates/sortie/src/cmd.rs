//! Command builders. Every write is a [`Cmd<B>`]: common options (`key`, `timeout`, `after`,
//! `lane`, `on_pause`) on all of them, body-specific setters on `Cmd<SpecificBody>`, and
//! `.await` (via `IntoFuture`) sends it and waits for the result from DCS.

use std::future::IntoFuture;
use std::marker::PhantomData;
use std::panic::Location;
use std::rc::Rc;
use std::time::Duration;

use futures::future::LocalBoxFuture;
use sortie_proto::v1::{self, command::Body};
use sortie_types::{AirbaseRef, Eid, Kind, Outcome, SmokeColor, StartType, Vec3};

use crate::ctx::{MessageTarget, Position};
use crate::group::{AiOption, Group};
use crate::menu::{MenuCommand, MenuScope};
use crate::runtime::{Session, Stop};
use crate::task::Task;
use crate::{Error, Failure, Result};

/// A command body: how to build the proto body and decode the result.
pub trait CommandBody: Sized + 'static {
    type Output;
    /// SDK operation name used in errors, e.g. `"spawn_group"`.
    const OP: &'static str;
    /// Dev mode (`sortie dev`, FakeDcs) refuses this command without `.key()`: hot reload
    /// re-runs the mission, and only a key stops it from spawning or destroying twice.
    const KEY_REQUIRED_IN_DEV: bool = false;
    fn to_proto(&self, session: &Session) -> Result<Body>;
    fn decode(self, session: &Rc<Session>, result: v1::CommandResult) -> Result<Self::Output>;
    /// What the command is aimed at, for error messages (e.g. `group "Red CAP 1"`).
    fn target(&self) -> String {
        String::new()
    }
    /// The output when the session ends before the result arrives; `None` = the command
    /// fails with `Error::SessionEnded`. `end_session` succeeds this way.
    fn on_session_ended(&self) -> Option<Self::Output> {
        None
    }
}

/// A command waiting to be sent. Nothing happens until it is awaited.
#[must_use = "commands do nothing unless `.await`ed"]
pub struct Cmd<B> {
    session: Rc<Session>,
    id: String,
    key: String,
    timeout: Option<Duration>,
    after: String,
    lane: v1::Lane,
    on_pause: v1::PausePolicy,
    origin: &'static Location<'static>,
    pub(crate) body: B,
}

impl<B> std::fmt::Debug for Cmd<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cmd")
            .field("id", &self.id)
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

impl<B> Cmd<B> {
    pub(crate) fn new(session: Rc<Session>, body: B, origin: &'static Location<'static>) -> Self {
        let id = session.next_command_id();
        Self {
            session,
            id,
            key: String::new(),
            timeout: None,
            after: String::new(),
            lane: v1::Lane::Unspecified,
            on_pause: v1::PausePolicy::Unspecified,
            origin,
            body,
        }
    }

    /// Command id, `"<session4>-<proc>-<n>"`: unique across logic processes and stable across
    /// reconnects; use with [`Cmd::after`].
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Idempotency key (scope: session). A command whose key already ran returns the stored
    /// result without reaching DCS.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = key.into();
        self
    }

    /// Fails with `Error::Timeout` if DCS has not answered within `dur` (engine side).
    pub fn timeout(mut self, dur: Duration) -> Self {
        self.timeout = Some(dur);
        self
    }

    /// Run only after `other` completed.
    pub fn after<C>(mut self, other: &Cmd<C>) -> Self {
        self.after.clone_from(&other.id);
        self
    }

    pub fn lane(mut self, lane: v1::Lane) -> Self {
        self.lane = lane;
        self
    }

    /// What happens while the simulation is paused (default: wait).
    pub fn on_pause(mut self, policy: v1::PausePolicy) -> Self {
        self.on_pause = policy;
        self
    }

    fn origin_string(&self) -> String {
        format!(
            "{}:{}:{}",
            self.origin.file(),
            self.origin.line(),
            self.origin.column()
        )
    }

    fn origin(&self) -> v1::Origin {
        v1::Origin {
            logic: self.session.logic.clone(),
            build_id: self.session.build_id.clone(),
            file: self.origin.file().to_owned(),
            line: self.origin.line(),
            column: self.origin.column(),
        }
    }
}

impl<B: CommandBody> Cmd<B> {
    /// The failure context of this command (op, target, ids, call site), without a message.
    fn failure(&self) -> Failure {
        Failure {
            op: B::OP,
            target: self.body.target(),
            command_id: self.id.clone(),
            session: self.session.id.clone(),
            message: String::new(),
            lua_traceback: None,
            origin: Some(self.origin_string()),
        }
    }

    async fn run(self) -> Result<B::Output> {
        if self.session.dev && B::KEY_REQUIRED_IN_DEV && self.key.is_empty() {
            return Err(Error::Invalid(Box::new(Failure {
                message: format!(
                    "`{}` without `.key(..)` in dev mode: a hot reload would run it again; \
                     add a key such as `.key(\"{}-1\")`",
                    B::OP,
                    B::OP.replace('_', "-")
                ),
                ..self.failure()
            })));
        }
        let body = self
            .body
            .to_proto(&self.session)
            .map_err(|e| with_origin(e, &self.origin_string()))?;
        let cmd = v1::Command {
            id: self.id.clone(),
            key: self.key.clone(),
            lane: self.lane as i32,
            timeout_ms: self
                .timeout
                .map_or(0, |d| u32::try_from(d.as_millis()).unwrap_or(u32::MAX)),
            on_pause: self.on_pause as i32,
            after: self.after.clone(),
            origin: Some(self.origin()),
            session: String::new(),
            body: Some(body),
        };
        let result = match self.session.submit(cmd).await {
            Ok(result) => result,
            Err(stop) => {
                if matches!(stop, Stop::Ended(_))
                    && let Some(out) = self.body.on_session_ended()
                {
                    return Ok(out);
                }
                return Err(stop.into_error(self.failure()));
            }
        };
        if let Some(v1::command_result::Outcome::Err(e)) = &result.outcome {
            if e.code == v1::ErrorCode::SessionEnded as i32
                && let Some(out) = self.body.on_session_ended()
            {
                return Ok(out);
            }
            return Err(Error::from_command_error(B::OP, self.failure(), e));
        }
        let failure = self.failure();
        self.body
            .decode(&self.session, result)
            .map_err(|e| match e {
                Error::Invalid(f) => Error::Invalid(Box::new(Failure {
                    message: f.message,
                    ..failure
                })),
                other => other,
            })
    }
}

/// Puts the call site on lookup errors raised while building a command.
fn with_origin(e: Error, at: &str) -> Error {
    match e {
        Error::UnknownSymbol { kind, ident, .. } => Error::UnknownSymbol {
            kind,
            ident,
            origin: Some(at.to_owned()),
        },
        Error::NotInWorld { kind, ident, .. } => Error::NotInWorld {
            kind,
            ident,
            origin: Some(at.to_owned()),
        },
        other => other,
    }
}

impl<B: CommandBody> IntoFuture for Cmd<B> {
    type Output = Result<B::Output>;
    type IntoFuture = LocalBoxFuture<'static, Result<B::Output>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.run())
    }
}

fn unit(_: &Rc<Session>, _: v1::CommandResult) -> Result<()> {
    Ok(())
}

// ----- bodies ---------------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SetOption {
    pub(crate) group: v1::GroupSel,
    pub(crate) group_name: String,
    pub(crate) option: AiOption,
}

impl CommandBody for SetOption {
    type Output = ();
    const OP: &'static str = "set_option";
    fn target(&self) -> String {
        format!("group {:?}", self.group_name)
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        let option = match self.option {
            AiOption::Alarm(a) => v1::set_option::Option::AlarmState(a.as_i32()),
            AiOption::Roe(r) => v1::set_option::Option::Roe(r.as_i32()),
        };
        Ok(Body::SetOption(v1::SetOption {
            group: Some(self.group.clone()),
            option: Some(option),
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<()> {
        unit(s, r)
    }
}

#[derive(Clone, Debug)]
pub struct SendMessage {
    to: MessageTarget,
    text: String,
    seconds: u32,
    clear_view: bool,
}

impl SendMessage {
    pub(crate) fn new(to: MessageTarget) -> Self {
        Self {
            to,
            text: String::new(),
            seconds: 10,
            clear_view: false,
        }
    }
}

impl Cmd<SendMessage> {
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.body.text = text.into();
        self
    }

    /// Display time in seconds (default 10).
    pub fn secs(mut self, seconds: u32) -> Self {
        self.body.seconds = seconds;
        self
    }

    pub fn clear_view(mut self) -> Self {
        self.body.clear_view = true;
        self
    }
}

impl CommandBody for SendMessage {
    type Output = ();
    const OP: &'static str = "message";
    fn target(&self) -> String {
        match &self.to {
            MessageTarget::All => "everyone".into(),
            MessageTarget::Coalition(c) => format!("coalition {c:?}"),
            MessageTarget::Group { name, .. } => format!("group {name:?}"),
        }
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        let to = match &self.to {
            MessageTarget::All => v1::message::To::All(true),
            MessageTarget::Coalition(c) => v1::message::To::Coalition(c.as_i32()),
            MessageTarget::Group { sel, .. } => v1::message::To::Group(sel.clone()),
        };
        Ok(Body::Message(v1::Message {
            to: Some(to),
            text: self.text.clone(),
            seconds: self.seconds,
            clear_view: self.clear_view,
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<()> {
        unit(s, r)
    }
}

#[derive(Clone, Debug)]
pub struct Smoke {
    pub(crate) at: Position,
    pub(crate) color: SmokeColor,
}

impl CommandBody for Smoke {
    type Output = ();
    const OP: &'static str = "smoke";
    fn to_proto(&self, _: &Session) -> Result<Body> {
        let (at, ground) = match self.at {
            Position::Ground(p) => (Vec3::new(p.north, 0.0, p.east), true),
            Position::At(v) => (v, false),
        };
        Ok(Body::Effect(v1::Effect {
            kind: Some(v1::effect::Kind::Smoke(v1::Smoke {
                at: Some(at.into()),
                color: self.color.as_i32(),
                at_ground_level: ground,
            })),
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<()> {
        unit(s, r)
    }
}

#[derive(Clone, Debug)]
pub struct SetFlag {
    pub(crate) ident: &'static str,
    pub(crate) value: i32,
}

impl CommandBody for SetFlag {
    type Output = ();
    const OP: &'static str = "flag_set";
    fn target(&self) -> String {
        format!("flag `{}`", self.ident)
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Flag(v1::Flag {
            ident: self.ident.to_owned(),
            value: self.value,
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<()> {
        unit(s, r)
    }
}

#[derive(Clone, Debug)]
pub struct GetFlag {
    pub(crate) ident: &'static str,
}

impl CommandBody for GetFlag {
    type Output = i32;
    const OP: &'static str = "flag_get";
    fn target(&self) -> String {
        format!("flag `{}`", self.ident)
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Query(v1::Query {
            kind: Some(v1::query::Kind::FlagValue(v1::FlagValue {
                ident: self.ident.to_owned(),
            })),
        }))
    }
    fn decode(self, _: &Rc<Session>, r: v1::CommandResult) -> Result<i32> {
        match r.outcome {
            Some(v1::command_result::Outcome::Ok(v)) => v
                .as_i64()
                .and_then(|n| i32::try_from(n).ok())
                .ok_or_else(|| {
                    Error::Invalid(Box::new(Failure::new(
                        Self::OP,
                        format!("flag value is not an integer: {v}"),
                    )))
                }),
            _ => Ok(0),
        }
    }
}

/// `ctx.spawn_group(template)`; place it with `.from_airbase()` or `.at()`.
#[derive(Clone, Debug)]
pub struct SpawnGroup<K> {
    template: &'static str,
    name: Option<String>,
    placement: Option<v1::spawn_group::Placement>,
    _k: PhantomData<fn() -> K>,
}

impl<K> SpawnGroup<K> {
    pub(crate) fn new(template: &'static str, name: Option<String>) -> Self {
        Self {
            template,
            name,
            placement: None,
            _k: PhantomData,
        }
    }
}

impl<K: Kind> Cmd<SpawnGroup<K>> {
    /// Explicit runtime name (default `<template name>#<session4>-<n>`); `NameConflict` if taken.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.body.name = Some(name.into());
        self
    }

    pub fn from_airbase<M>(mut self, airbase: AirbaseRef<M>, start: StartType) -> Self {
        self.body.placement = Some(v1::spawn_group::Placement::FromAirbase(v1::FromAirbase {
            airbase: airbase.ident().to_owned(),
            start: start.as_i32(),
        }));
        self
    }

    /// Spawn at `position` (the template's own route is shifted there).
    pub fn at(mut self, position: Vec3, heading_rad: f64) -> Self {
        self.body.placement = Some(v1::spawn_group::Placement::At(v1::SpawnAt {
            position: Some(position.into()),
            heading_rad,
        }));
        self
    }
}

impl<K: Kind> CommandBody for SpawnGroup<K> {
    type Output = Group<K>;
    const OP: &'static str = "spawn_group";
    const KEY_REQUIRED_IN_DEV: bool = true;
    fn target(&self) -> String {
        match &self.name {
            Some(name) => format!("template `{}` as {name:?}", self.template),
            None => format!("template `{}`", self.template),
        }
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::SpawnGroup(v1::SpawnGroup {
            template: self.template.to_owned(),
            name: self.name.clone(),
            placement: self.placement.clone(),
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<Group<K>> {
        let eid = Eid::from_u64(r.eid).ok_or_else(|| {
            Error::Invalid(Box::new(Failure::new(
                Self::OP,
                "the engine returned no group eid",
            )))
        })?;
        let name = match &r.outcome {
            Some(v1::command_result::Outcome::Ok(v)) => {
                v.get("name").and_then(|n| n.as_str()).map(str::to_owned)
            }
            _ => None,
        };
        Ok(Group::new(
            s.clone(),
            eid,
            name.or(self.name).unwrap_or_default(),
        ))
    }
}

#[derive(Clone, Debug)]
pub struct SetTask {
    pub(crate) group: v1::GroupSel,
    pub(crate) group_name: String,
    pub(crate) push: bool,
    pub(crate) task: Task,
}

impl CommandBody for SetTask {
    type Output = ();
    const OP: &'static str = "set_task";
    fn target(&self) -> String {
        format!("group {:?}", self.group_name)
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::SetTask(v1::SetTask {
            group: Some(self.group.clone()),
            push: self.push,
            task: Some(self.task.to_proto()),
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<()> {
        unit(s, r)
    }
}

#[derive(Clone, Debug)]
pub struct Destroy {
    pub(crate) group: v1::GroupSel,
    pub(crate) group_name: String,
}

impl CommandBody for Destroy {
    type Output = ();
    const OP: &'static str = "destroy";
    const KEY_REQUIRED_IN_DEV: bool = true;
    fn target(&self) -> String {
        format!("group {:?}", self.group_name)
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Destroy(v1::Destroy {
            target: Some(v1::destroy::Target::Group(self.group.clone())),
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<()> {
        unit(s, r)
    }
}

/// Adds a menu command; resolves to a [`MenuCommand`] whose presses can be awaited.
#[derive(Clone, Debug)]
pub struct AddMenu<S> {
    pub(crate) coalition: sortie_types::Coalition,
    pub(crate) ident: &'static str,
    pub(crate) _s: PhantomData<fn() -> S>,
}

impl<S: MenuScope> CommandBody for AddMenu<S> {
    type Output = MenuCommand<S>;
    const OP: &'static str = "menu_add";
    fn target(&self) -> String {
        format!("menu `{}`", self.ident)
    }
    fn to_proto(&self, session: &Session) -> Result<Body> {
        let menu = sortie_manifest::menu(&session.manifest, self.ident)
            .ok_or_else(|| Error::unknown_symbol(sortie_types::SymbolKind::Menu, self.ident))?;
        Ok(Body::Menu(v1::Menu {
            scope: S::SCOPE as i32,
            coalition: self.coalition.as_i32(),
            ident: self.ident.to_owned(),
            label: menu.label.clone(),
            path: menu.path.clone(),
            remove: false,
        }))
    }
    fn decode(self, s: &Rc<Session>, _: v1::CommandResult) -> Result<MenuCommand<S>> {
        Ok(MenuCommand::new(s.clone(), self.coalition, self.ident))
    }
}

#[derive(Clone, Debug)]
pub struct RemoveMenu {
    pub(crate) scope: v1::MenuScope,
    pub(crate) coalition: sortie_types::Coalition,
    pub(crate) ident: &'static str,
}

impl CommandBody for RemoveMenu {
    type Output = ();
    const OP: &'static str = "menu_remove";
    fn target(&self) -> String {
        format!("menu `{}`", self.ident)
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Menu(v1::Menu {
            scope: self.scope as i32,
            coalition: self.coalition.as_i32(),
            ident: self.ident.to_owned(),
            label: String::new(),
            path: Vec::new(),
            remove: true,
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<()> {
        unit(s, r)
    }
}

#[derive(Clone, Debug)]
pub struct EndSession {
    pub(crate) outcome: Outcome,
    pub(crate) text: String,
}

impl Cmd<EndSession> {
    /// Text shown to players / stored in the results.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.body.text = text.into();
        self
    }
}

impl CommandBody for EndSession {
    type Output = ();
    const OP: &'static str = "end_session";
    fn on_session_ended(&self) -> Option<()> {
        Some(())
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::EndSession(v1::EndSession {
            outcome: self.outcome.as_i32(),
            text: self.text.clone(),
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<()> {
        unit(s, r)
    }
}
