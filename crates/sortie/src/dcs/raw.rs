//! The untyped universal call (cargo feature `dcs-raw`): any catalog path by its string name,
//! arguments built by hand, the raw [`Return`]; and batches.
//!
//! ```ignore
//! use sortie::dcs::arg;
//! let r = ctx.dcs().call("land.getHeight").arg(arg::vec2(p)).await?;
//! let h: f64 = r.first().as_f64().unwrap_or_default();
//! let out = ctx.dcs().batch()
//!     .push(ctx.dcs().call("Airbase.getWarehouse").on(arg::airbase(base)))
//!     .push(ctx.dcs().call("Warehouse.getItemCount").on_result_of(0).arg(arg::string(item)))
//!     .await?;
//! ```
//!
//! The engine checks a raw call exactly like a wrapper's: the path must be in the catalog and
//! enabled, the arguments must fit the entry's signature.

use std::panic::Location;
use std::rc::Rc;

use sortie_proto::v1::{self, command::Body};

use super::{Dcs, Return, SINCE_API_REV, no_typed_result};
use crate::cmd::{Cmd, CommandBody};
use crate::runtime::Session;
use crate::{Error, Failure, Result};

/// `ctx.dcs().call(path)`: one call by path.
#[derive(Clone, Debug)]
pub struct RawCall {
    call: v1::DcsCall,
}

impl Dcs {
    /// A call of the catalog path `path` ("land.getHeight", "Unit.getFuel", ...).
    #[track_caller]
    pub fn call(&self, path: impl Into<String>) -> Cmd<RawCall> {
        Cmd::new(
            self.session.clone(),
            RawCall {
                call: v1::DcsCall {
                    path: path.into(),
                    ..Default::default()
                },
            },
            Location::caller(),
        )
    }

    /// Several calls in one frame, validated as a whole before any runs.
    #[track_caller]
    pub fn batch(&self) -> Cmd<Batch> {
        Cmd::new(
            self.session.clone(),
            Batch {
                batch: v1::DcsBatch::default(),
            },
            Location::caller(),
        )
    }
}

impl Cmd<RawCall> {
    /// Runs in the hooks environment (for a path listed in both; otherwise the entry decides).
    pub fn hooks(mut self) -> Self {
        self.body.call.env = v1::Env::Hooks as i32;
        self
    }

    /// The method's receiver (see [`super::arg`] and [`super::recv`]).
    pub fn on(mut self, receiver: v1::DcsRef) -> Self {
        self.body.call.receiver = Some(receiver);
        self
    }

    /// In a batch: the receiver is the first value of call `index` (0-based).
    pub fn on_result_of(mut self, index: u32) -> Self {
        self.body.call.receiver = Some(v1::DcsRef {
            target: Some(v1::dcs_ref::Target::ResultOf(index)),
        });
        self
    }

    /// The next argument.
    pub fn arg(mut self, a: v1::DcsArg) -> Self {
        self.body.call.args.push(a);
        self
    }

    /// Depth to which returned tables are copied (default 6, at most 12).
    pub fn max_depth(mut self, depth: u32) -> Self {
        self.body.call.max_depth = depth;
        self
    }
}

impl CommandBody for RawCall {
    type Output = Return;
    const OP: &'static str = "dcs_call";
    const SINCE_API_REV: u32 = SINCE_API_REV;
    fn target(&self) -> String {
        format!("`{}`", self.call.path)
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::DcsCall(self.call.clone()))
    }
    fn decode(self, _: &Rc<Session>, r: v1::CommandResult) -> Result<Return> {
        let replayed = r.replayed;
        r.reply
            .and_then(|x| x.call)
            .map(Return::from_proto)
            .ok_or_else(|| no_typed_result(replayed, "Reply.call"))
    }
}

/// `ctx.dcs().batch()`.
#[derive(Clone, Debug)]
pub struct Batch {
    batch: v1::DcsBatch,
}

impl Cmd<Batch> {
    /// Adds a call built with `ctx.dcs().call(..)` (its own key, timeout and `after` are not
    /// used: the batch is one command).
    pub fn push(mut self, call: Cmd<RawCall>) -> Self {
        self.body.batch.calls.push(call.body.call);
        self
    }

    /// The first failing call stops the batch; the rest are skipped.
    pub fn stop_on_error(mut self) -> Self {
        self.body.batch.stop_on_error = true;
        self
    }

    /// A failing call fails the command (with that call's error; nothing is stored under the
    /// key). Without it the command succeeds and each result says what happened.
    pub fn fail_on_error(mut self) -> Self {
        self.body.batch.fail_command_on_error = true;
        self
    }
}

/// One call of a batch.
#[derive(Debug)]
pub enum CallResult {
    Ok(Return),
    Err(Error),
    /// Not run: an earlier call failed (`stop_on_error`), or its `on_result_of` call gave no
    /// object.
    Skipped,
}

/// What a batch returned, in call order.
#[derive(Debug)]
pub struct BatchReturn {
    pub results: Vec<CallResult>,
    /// DCS time the whole batch took in the Lua layer, microseconds: the batch runs in one
    /// frame, so this is its share of that frame (`CommandResult.cost_us`).
    pub cost_us: u32,
}

impl CommandBody for Batch {
    type Output = BatchReturn;
    const OP: &'static str = "dcs_batch";
    const SINCE_API_REV: u32 = SINCE_API_REV;
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::DcsBatch(self.batch.clone()))
    }
    fn decode(self, _: &Rc<Session>, r: v1::CommandResult) -> Result<BatchReturn> {
        let (cost_us, replayed) = (r.cost_us, r.replayed);
        let b = r
            .reply
            .and_then(|x| x.batch)
            .ok_or_else(|| no_typed_result(replayed, "Reply.batch"))?;
        let results = b
            .results
            .into_iter()
            .zip(&self.batch.calls)
            .map(|(x, c)| match x.outcome {
                Some(v1::dcs_call_result::Outcome::Ok(ret)) => {
                    CallResult::Ok(Return::from_proto(ret))
                }
                Some(v1::dcs_call_result::Outcome::Err(e)) => {
                    CallResult::Err(Error::from_command_error(
                        "dcs_call",
                        Failure {
                            target: format!("`{}`", c.path),
                            ..Failure::new("dcs_call", "")
                        },
                        &e,
                    ))
                }
                None => CallResult::Skipped,
            })
            .collect();
        Ok(BatchReturn { results, cost_us })
    }
}
