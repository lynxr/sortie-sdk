//! `ctx.api()`: what this logic serves through the engine's web API (API revision 3). A logic
//! declares named actions; a web site or a third-party tool (a token with the role `operator`
//! or `admin`) calls one with `POST /api/v1/actions/<logic>/<name>` and a JSON body; the
//! request arrives here, and the logic's answer is the HTTP response. The logic decides what
//! happens in the game: a web caller never sends DCS commands itself.
//!
//! ```ignore
//! #[derive(Deserialize)]
//! struct Move { x: f64, z: f64 }
//!
//! #[sortie::mission(api_rev = 3)]
//! async fn run(ctx: Ctx<OpNorth>, _start: Start) -> sortie::Result<()> {
//!     let mut moves = ctx.api().action("move_convoy")?;
//!     while let Some(req) = moves.next().await {
//!         let to: Move = match req.json() {
//!             Ok(to) => to,
//!             Err(e) => { req.fail(400, e)?; continue; }
//!         };
//!         // ... task the convoy ...
//!         req.reply(&serde_json::json!({"moving": true}))?;   // HTTP 200 with this JSON
//!     }
//!     Ok(())
//! }
//! ```
//!
//! * [`Api::action`] declares the action at once and returns the stream of its requests; the
//!   engine routes calls to this logic from then on. Dropping the stream (or the end of the
//!   session) withdraws the declaration; a restarted logic declares again when its mission
//!   function runs. Between sessions a caller gets 503 (`unavailable`).
//! * Each [`ActionRequest`] is answered once: [`ActionRequest::reply`] (200 with JSON) or
//!   [`ActionRequest::fail`] (a status 400 to 499 and a message). A request dropped without an
//!   answer is answered 500 (`unanswered`) by the SDK. The engine answers 504 itself when the
//!   logic takes more than 10 s ([`ACTION_TIMEOUT`]), so answer first and do long work
//!   afterwards. A request that waited that long in the queue (the logic did not read the
//!   stream) is never handed to the logic: the SDK answers it `unanswered` when a newer one
//!   arrives or when the logic reads, so the logic never runs what its caller was told timed out.
//! * Rules, else `Error::Invalid` and nothing is declared: an action name is 1 to 64 bytes of
//!   `a-z`, `0-9`, `_`, `-`; at most 64 actions; one stream per action at a time; the logic's
//!   own name (its binary's name, `Attach.logic`) is 1 to 64 bytes of `A-Z`, `a-z`, `0-9`, `.`,
//!   `_`, `-` (it is a path segment of the URL).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Display;
use std::panic::Location;
use std::rc::Rc;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use sortie_proto::v1::{self, action_reply::Answer, logic_up};
use tokio::sync::{Notify, mpsc};
use tokio::time::Instant;

use crate::cmd::check_since;
use crate::runtime::Session;
use crate::{Error, Failure, Result};

/// The API revision of the web API's actions.
pub const SINCE_API_REV: u32 = 3;
/// How many actions one logic may declare at a time.
pub const MAX_ACTIONS: usize = 64;
/// The longest action name (bytes of `a-z0-9_-`).
pub const MAX_ACTION_NAME: usize = 64;
/// The largest answer (bytes of JSON); a larger one is not sent ([`ActionRequest::reply`]).
pub const MAX_REPLY_BYTES: usize = 1 << 20;
/// How long the engine waits for the logic's answer before it answers the caller 504; a
/// request that waited this long in the queue is answered `unanswered` and never handed to the
/// logic ([`ActionRequests::next`]).
pub const ACTION_TIMEOUT: Duration = Duration::from_secs(10);

/// What this logic serves through the engine's web API ([`crate::Ctx::api`]).
pub struct Api {
    session: Rc<Session>,
}

impl std::fmt::Debug for Api {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Api").finish_non_exhaustive()
    }
}

/// An action name: 1 to [`MAX_ACTION_NAME`] bytes of `a-z`, `0-9`, `_`, `-`.
fn check_name(name: &str) -> std::result::Result<(), String> {
    let ok = (1..=MAX_ACTION_NAME).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(format!(
            "`{name}` is not an action name: an action name is 1 to {MAX_ACTION_NAME} bytes of \
             a-z, 0-9, `_` and `-`"
        ))
    }
}

/// The logic's own name is a path segment of the action's URL.
fn check_logic(logic: &str) -> std::result::Result<(), String> {
    let ok = (1..=64).contains(&logic.len())
        && logic
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if ok {
        Ok(())
    } else {
        Err(format!(
            "this logic is named `{logic}` (its binary's name, Attach.logic), which cannot be a \
             path segment of /api/v1/actions/<logic>/<name>: a logic that declares actions is \
             named with 1 to 64 bytes of A-Z, a-z, 0-9, `.`, `_` and `-` (rename the binary or \
             pass --logic)"
        ))
    }
}

fn site(origin: &'static Location<'static>) -> String {
    format!("{}:{}:{}", origin.file(), origin.line(), origin.column())
}

impl Api {
    pub(crate) fn new(session: Rc<Session>) -> Self {
        Self { session }
    }

    /// Declares the action `name` and returns its requests: from now on the engine hands
    /// `POST /api/v1/actions/<logic>/<name>` to this logic. Dropping the stream withdraws it;
    /// so does the end of the session. API revision 3: declare
    /// `#[sortie::mission(api_rev = 3)]`. See [`crate::api`].
    ///
    /// # Errors
    /// `Error::Revision` / `Error::EngineTooOld` below revision 3; `Error::Invalid` for a name
    /// outside the rules, a name declared already (by a stream still alive), more than
    /// [`MAX_ACTIONS`], or a logic name that cannot be a path segment; `Error::SessionEnded`
    /// after the session. Nothing is declared then.
    #[track_caller]
    pub fn action(&self, name: &str) -> Result<ActionRequests> {
        let origin = Location::caller();
        let s = &self.session;
        let failure = Failure {
            op: "action",
            target: format!("action \"{name}\""),
            session: s.id.clone(),
            origin: Some(site(origin)),
            ..Failure::default()
        };
        let invalid = |message: String| {
            Error::Invalid(Box::new(Failure {
                message,
                ..failure.clone()
            }))
        };
        check_since(
            s.declared.api_rev,
            s.engine_api_rev,
            SINCE_API_REV,
            failure.clone(),
        )?;
        check_logic(&s.logic).map_err(invalid)?;
        check_name(name).map_err(invalid)?;
        if s.is_ended() {
            return Err(Error::SessionEnded(Box::new(Failure {
                message: format!("session {} has ended", s.id),
                ..failure
            })));
        }
        let inbox = Rc::new(ActionInbox {
            name: name.to_owned(),
            origin: site(origin),
            queue: RefCell::default(),
            notify: Notify::new(),
        });
        s.declare_action(&inbox).map_err(invalid)?;
        Ok(ActionRequests {
            session: s.clone(),
            inbox,
        })
    }
}

/// Requests of one action waiting to be read (the session holds it weakly).
#[derive(Debug)]
pub(crate) struct ActionInbox {
    pub(crate) name: String,
    /// Where the action was declared (for a second declaration's error).
    pub(crate) origin: String,
    queue: RefCell<VecDeque<ActionRequest>>,
    notify: Notify,
}

impl ActionInbox {
    /// Queues `req`; the requests the engine gave up on leave the queue first.
    pub(crate) fn offer(&self, req: ActionRequest) {
        let stale = {
            let mut queue = self.queue.borrow_mut();
            let stale = take_stale(&mut queue);
            queue.push_back(req);
            stale
        };
        expire(stale);
        self.notify.notify_one();
    }

    /// The oldest request the engine still waits for.
    fn pop(&self) -> Option<ActionRequest> {
        let (next, stale) = {
            let mut queue = self.queue.borrow_mut();
            let stale = take_stale(&mut queue);
            (queue.pop_front(), stale)
        };
        expire(stale);
        next
    }
}

/// The requests at the front of `queue` (oldest first) that waited [`ACTION_TIMEOUT`] or longer.
fn take_stale(queue: &mut VecDeque<ActionRequest>) -> Vec<ActionRequest> {
    let mut stale = Vec::new();
    while queue
        .front()
        .is_some_and(|r| r.arrived.elapsed() >= ACTION_TIMEOUT)
    {
        stale.extend(queue.pop_front());
    }
    stale
}

/// Answers requests the engine gave up on (outside the queue's borrow).
fn expire(stale: Vec<ActionRequest>) {
    for mut req in stale {
        let why = format!(
            "the request of action `{}` waited {} s in the logic's queue, and the engine waits {} \
             s for an answer: the caller got 504 already, so the logic never saw the request",
            req.name,
            req.arrived.elapsed().as_secs(),
            ACTION_TIMEOUT.as_secs()
        );
        req.send(Answer::Unanswered(why));
    }
}

/// The requests of one declared action ([`Api::action`]); dropping it withdraws the action
/// (requests still queued are answered 500).
pub struct ActionRequests {
    session: Rc<Session>,
    inbox: Rc<ActionInbox>,
}

impl std::fmt::Debug for ActionRequests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActionRequests")
            .field("name", &self.inbox.name)
            .finish_non_exhaustive()
    }
}

impl ActionRequests {
    /// The action's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.inbox.name
    }

    /// The next request; `None` once the session ended. A request that waited
    /// [`ACTION_TIMEOUT`] or longer is skipped (the engine answered its caller 504 already)
    /// and answered `unanswered`.
    pub async fn next(&mut self) -> Option<ActionRequest> {
        let mut end = self.session.end.subscribe();
        loop {
            if let Some(req) = self.inbox.pop() {
                return Some(req);
            }
            if end.borrow_and_update().is_some() {
                return None;
            }
            tokio::select! {
                () = self.inbox.notify.notified() => {}
                r = end.changed() => {
                    if r.is_err() {
                        return None;
                    }
                }
            }
        }
    }
}

impl Drop for ActionRequests {
    fn drop(&mut self) {
        self.session.withdraw_action(&self.inbox.name);
    }
}

/// One call of an action: its JSON body and who called. Answer it once with
/// [`ActionRequest::reply`] or [`ActionRequest::fail`]; dropped without an answer, the SDK
/// answers it 500 (`unanswered`).
pub struct ActionRequest {
    id: u64,
    name: String,
    json: Vec<u8>,
    token: String,
    role: String,
    up: mpsc::UnboundedSender<v1::LogicUp>,
    answered: bool,
    /// When it reached this logic (the engine's wait began just before).
    arrived: Instant,
}

impl std::fmt::Debug for ActionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActionRequest")
            .field("name", &self.name)
            .field("token", &self.token)
            .field("role", &self.role)
            .finish_non_exhaustive()
    }
}

impl ActionRequest {
    pub(crate) fn new(call: v1::ActionCall, up: mpsc::UnboundedSender<v1::LogicUp>) -> Self {
        Self {
            id: call.id,
            name: call.name,
            json: call.json,
            token: call.token,
            role: call.role,
            up,
            answered: false,
            arrived: Instant::now(),
        }
    }

    /// The action called.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The request body as sent: JSON (`null` for an empty body).
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.json
    }

    /// The body as `T` (serde_json).
    ///
    /// # Errors
    /// `Error::Invalid` when it does not fit `T`; its message says why, so
    /// `req.fail(400, e)` tells the caller.
    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_slice(&self.json).map_err(|e| {
            Error::Invalid(Box::new(Failure {
                op: "action",
                target: format!("action \"{}\"", self.name),
                message: format!("the request body does not fit: {e}"),
                ..Failure::default()
            }))
        })
    }

    /// The name of the caller's API token (`None` without one, or for a token without a name).
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        Some(self.token.as_str()).filter(|t| !t.is_empty())
    }

    /// The caller's role: `"operator"` or `"admin"`.
    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
    }

    fn send(&mut self, answer: Answer) -> bool {
        self.answered = true;
        self.up
            .send(v1::LogicUp {
                msg: Some(logic_up::Msg::ActionReply(v1::ActionReply {
                    id: self.id,
                    answer: Some(answer),
                })),
            })
            .is_ok()
    }

    fn failure(&self, origin: &'static Location<'static>, message: String) -> Error {
        Error::Invalid(Box::new(Failure {
            op: "action",
            target: format!("action \"{}\"", self.name),
            message,
            origin: Some(site(origin)),
            ..Failure::default()
        }))
    }

    /// Answers 200 with `value` as JSON (at most [`MAX_REPLY_BYTES`]).
    ///
    /// # Errors
    /// `Error::Invalid` when `value` cannot be written as JSON or is too large (the caller then
    /// gets 500 `unanswered`); `Error::Transport` when the engine stream is gone.
    #[track_caller]
    pub fn reply<T: Serialize + ?Sized>(mut self, value: &T) -> Result<()> {
        let origin = Location::caller();
        let json = match serde_json::to_vec(value) {
            Ok(j) if j.len() <= MAX_REPLY_BYTES => j,
            Ok(j) => {
                let message = format!(
                    "the answer is {} bytes of JSON; an answer is at most {MAX_REPLY_BYTES}",
                    j.len()
                );
                self.send(Answer::Unanswered(message.clone()));
                return Err(self.failure(origin, message));
            }
            Err(e) => {
                let message = format!("the answer cannot be written as JSON: {e}");
                self.send(Answer::Unanswered(message.clone()));
                return Err(self.failure(origin, message));
            }
        };
        if self.send(Answer::Json(json)) {
            Ok(())
        } else {
            Err(Error::Transport("the engine stream is closed".into()))
        }
    }

    /// Refuses the request: the caller gets `status` (400 to 499) and
    /// `{"error": {"code": "refused", "message": message}}`.
    ///
    /// # Errors
    /// `Error::Invalid` for a status outside 400..=499 (5xx statuses are the engine's; the
    /// caller then gets 500 `unanswered`); `Error::Transport` when the engine stream is gone.
    #[track_caller]
    pub fn fail(mut self, status: u16, message: impl Display) -> Result<()> {
        let origin = Location::caller();
        if !(400..=499).contains(&status) {
            let why = format!(
                "`fail` takes a status from 400 to 499 (5xx statuses are the engine's), not \
                 {status}"
            );
            self.send(Answer::Unanswered(why.clone()));
            return Err(self.failure(origin, why));
        }
        let refusal = v1::ActionRefusal {
            status: u32::from(status),
            message: message.to_string(),
        };
        if self.send(Answer::Refused(refusal)) {
            Ok(())
        } else {
            Err(Error::Transport("the engine stream is closed".into()))
        }
    }
}

impl Drop for ActionRequest {
    fn drop(&mut self) {
        if !self.answered {
            let why = format!(
                "the logic dropped the request of action `{}` without an answer (its handler \
                 ended or the session ended)",
                self.name
            );
            self.send(Answer::Unanswered(why));
        }
    }
}
