//! `ctx.publish(name, &value)`: named JSON documents of this logic that the engine serves
//! read-only over its web API, for a site's live map or campaign page (API revision 3).
//!
//! ```ignore
//! #[sortie::mission(api_rev = 3)]
//! async fn run(ctx: Ctx<OpNorth>, _start: Start) -> sortie::Result<()> {
//!     let p = ctx.publish("front", &Front { sectors }).await?; // Published { rev, changed, .. }
//!     let had = ctx.unpublish("old-front").await?;             // true when there was one
//!     Ok(())
//! }
//! ```
//!
//! * The engine keeps a document under the logic's name (`Attach.logic`: the binary's name)
//!   and serves it as `GET /api/v1/docs/<logic>/<name>` with its revision in the `ETag`, and as
//!   a stream of its changes at `.../events`. A request never reaches the logic or DCS.
//! * A document outlives the session, the logic process and the engine (it is in the engine's
//!   state database): after a restart the site still reads the last one; publish again what
//!   changed.
//! * `value` is written as JSON (serde_json) when `publish` is called; the future resolves when
//!   the engine stored it. The same JSON again changes nothing (`changed == false`, same
//!   `rev`), so a logic may publish on a timer.
//! * `rev` grows with every publish of any document of the engine: one revision is one content.
//! * Rules, else `Error::Invalid` and nothing is stored: a name is 1 to 64 bytes of `a-z`,
//!   `0-9`, `_`, `-`; a document is at most 256 KiB of JSON; a logic has at most 64 documents.
//!   The logic's own name must be 1 to 64 bytes of `A-Z`, `a-z`, `0-9`, `.`, `_`, `-`.
//! * The web API serves points in DCS metres; a map that needs latitude and longitude gets them
//!   from the logic's document (`ctx.dcs().coord().lo_to_ll(point)` converts, API revision 1).

use std::future::IntoFuture;
use std::panic::Location;
use std::rc::Rc;

use futures::future::LocalBoxFuture;
use serde::Serialize;
use sortie_proto::v1::{self, state_op::Op, state_result::Result as Answer};

use crate::cmd::check_since;
use crate::runtime::Session;
use crate::{Error, Failure, Result};

/// The API revision of published documents.
pub const SINCE_API_REV: u32 = 3;

/// What the engine stored ([`Publish`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Published {
    /// The engine-wide revision of the content now published (in the HTTP `ETag`, after the
    /// engine's store epoch: an engine without its database counts from 1 again).
    pub rev: u64,
    /// When that content was stored (engine wall clock), ms since 1970-01-01 UTC.
    pub updated_unix_ms: i64,
    /// `false`: the engine had these exact bytes already and changed nothing.
    pub changed: bool,
}

/// The failure context of a publish or unpublish.
fn failure(
    op: &'static str,
    session: &Session,
    name: &str,
    origin: &'static Location<'static>,
) -> Failure {
    Failure {
        op,
        target: format!("document \"{name}\""),
        session: session.id.clone(),
        origin: Some(format!(
            "{}:{}:{}",
            origin.file(),
            origin.line(),
            origin.column()
        )),
        ..Failure::default()
    }
}

/// The revision check: nothing is sent below revision 3 (the logic's or the engine's).
fn check(session: &Session, failure: &Failure) -> Result<()> {
    check_since(
        session.declared.api_rev,
        session.engine_api_rev,
        SINCE_API_REV,
        failure.clone(),
    )
}

/// Sends `op` (after [`check`]); the engine's `published` answer.
async fn send(session: &Session, op: Op, failure: Failure) -> Result<v1::PublishedDoc> {
    let answer = session.state_op(op).await.map_err(|e| match e {
        Error::SessionEnded(f) => Error::SessionEnded(Box::new(Failure {
            message: f.message,
            ..failure.clone()
        })),
        other => other,
    })?;
    match answer.result {
        Some(Answer::Published(p)) => Ok(p),
        Some(Answer::Err(e)) => Err(Error::from_command_error(failure.op, failure, &e)),
        other => Err(Error::Invalid(Box::new(Failure {
            message: format!("the engine answered {other:?}"),
            ..failure
        }))),
    }
}

/// A publish waiting to be sent ([`crate::Ctx::publish`]). Nothing happens until it is awaited.
#[must_use = "a publish does nothing unless `.await`ed"]
pub struct Publish {
    session: Rc<Session>,
    name: String,
    json: std::result::Result<Vec<u8>, String>,
    origin: &'static Location<'static>,
}

impl std::fmt::Debug for Publish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Publish")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl Publish {
    pub(crate) fn new<T: Serialize + ?Sized>(
        session: Rc<Session>,
        name: &str,
        value: &T,
        origin: &'static Location<'static>,
    ) -> Self {
        Self {
            session,
            name: name.to_owned(),
            json: serde_json::to_vec(value).map_err(|e| e.to_string()),
            origin,
        }
    }
}

impl IntoFuture for Publish {
    type Output = Result<Published>;
    type IntoFuture = LocalBoxFuture<'static, Result<Published>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let failure = failure("publish", &self.session, &self.name, self.origin);
            check(&self.session, &failure)?;
            let json = self.json.map_err(|e| {
                Error::Invalid(Box::new(Failure {
                    message: format!("the value cannot be written as JSON: {e}"),
                    ..failure.clone()
                }))
            })?;
            let op = Op::Publish(v1::PublishDoc {
                name: self.name.clone(),
                json,
            });
            let p = send(&self.session, op, failure).await?;
            Ok(Published {
                rev: p.rev,
                updated_unix_ms: p.updated_unix_ms,
                changed: p.changed,
            })
        })
    }
}

/// An unpublish waiting to be sent ([`crate::Ctx::unpublish`]); resolves to whether a document
/// was removed.
#[must_use = "an unpublish does nothing unless `.await`ed"]
pub struct Unpublish {
    session: Rc<Session>,
    name: String,
    origin: &'static Location<'static>,
}

impl std::fmt::Debug for Unpublish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Unpublish")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl Unpublish {
    pub(crate) fn new(
        session: Rc<Session>,
        name: &str,
        origin: &'static Location<'static>,
    ) -> Self {
        Self {
            session,
            name: name.to_owned(),
            origin,
        }
    }
}

impl IntoFuture for Unpublish {
    type Output = Result<bool>;
    type IntoFuture = LocalBoxFuture<'static, Result<bool>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let failure = failure("unpublish", &self.session, &self.name, self.origin);
            check(&self.session, &failure)?;
            let op = Op::Unpublish(v1::UnpublishDoc {
                name: self.name.clone(),
            });
            Ok(send(&self.session, op, failure).await?.changed)
        })
    }
}
