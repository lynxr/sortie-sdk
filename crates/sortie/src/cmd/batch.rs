//! SpawnBatch (API revision 1): many groups and statics in one command, the restore of a
//! persistent campaign.
//!
//! `ctx.spawn_batch()` collects spawns built as usual (`ctx.spawn_group(..)`,
//! `ctx.spawn_static(..)`, not awaited) and sends them as one command with one key. The engine
//! checks the whole batch before anything is sent (one bad item fails it, nothing spawns),
//! then the spawn gate spreads the items over time (2 groups and 24 units per second by
//! default): the result arrives when the last item settled, and the items in between are
//! already in the world. Nothing is rolled back: each item has its own result
//! ([`SpawnedBatch`]); the command fails only when it could not run (or its link dropped).
//!
//! ```ignore
//! let mut batch = ctx.spawn_batch().stop_on_error();
//! let cap = batch.add_group(ctx.spawn_group(templates::RED_CAP).named("CAP 1"));
//! let farp = batch.add_static(ctx.spawn_static("FARP", "USA").at(p, 0.0).heliport(Heliport::new()));
//! let out = batch.key("restore-turn-7").await?;
//! let cap: Group<Air> = out.get(&cap)?;
//! let farp: SpawnedStatic = out.get(&farp)?;
//! ```
//!
//! The options of an item's own command (`key`, `timeout`, `after`, `lane`) do not apply: the
//! batch has one key and one deadline (by default the lane's plus the time the gate needs).

use std::marker::PhantomData;
use std::rc::Rc;

use sortie_proto::v1::{self, command::Body};
use sortie_types::{Eid, Kind};

use super::objects::{SpawnStatic, SpawnedStatic};
use super::{Cmd, CommandBody, SpawnGroup};
use crate::group::Group;
use crate::runtime::Session;
use crate::{Error, Failure, Result};

/// The most items of one batch.
pub const MAX_BATCH_ITEMS: usize = 512;

/// `ctx.spawn_batch()`: add items with [`Cmd::group`] / [`Cmd::add_group`] and
/// [`Cmd::static_object`] / [`Cmd::add_static`].
#[derive(Clone, Debug, Default)]
pub struct SpawnBatch {
    items: Vec<v1::SpawnItem>,
    /// The first item that could not be built (an unknown template ident, ...).
    error: Option<Error>,
    stop_on_error: bool,
    since: u32,
}

impl SpawnBatch {
    pub(crate) fn new() -> Self {
        Self {
            since: 1,
            ..Self::default()
        }
    }

    fn push(&mut self, item: Result<v1::SpawnItem>, since: u32) -> usize {
        match item {
            Ok(i) => self.items.push(i),
            Err(e) => {
                self.error.get_or_insert(e);
                self.items.push(v1::SpawnItem::default());
            }
        }
        self.since = self.since.max(since);
        self.items.len() - 1
    }
}

/// A typed handle to one item of a batch: [`SpawnedBatch::get`] gives its result as `T`.
pub struct BatchSlot<T> {
    index: usize,
    _t: PhantomData<fn() -> T>,
}

impl<T> BatchSlot<T> {
    /// The item's position in the batch (0-based).
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }
}

impl<T> std::fmt::Debug for BatchSlot<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchSlot")
            .field("index", &self.index)
            .finish()
    }
}

impl<T> Clone for BatchSlot<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for BatchSlot<T> {}

fn slot<T>(index: usize) -> BatchSlot<T> {
    BatchSlot {
        index,
        _t: PhantomData,
    }
}

impl Cmd<SpawnBatch> {
    /// Adds a group spawn (built with `ctx.spawn_group(..)`, not awaited).
    pub fn group<K: Kind>(mut self, group: Cmd<SpawnGroup<K>>) -> Self {
        self.add_group(group);
        self
    }

    /// Adds a group spawn and returns its typed slot.
    pub fn add_group<K: Kind>(&mut self, group: Cmd<SpawnGroup<K>>) -> BatchSlot<Group<K>> {
        let since = group.body.since_api_rev();
        let item = group.body.to_proto(&group.session).and_then(|b| match b {
            Body::SpawnGroup(g) => Ok(v1::SpawnItem {
                spawn: Some(v1::spawn_item::Spawn::Group(g)),
            }),
            _ => Err(Error::Invalid(Box::new(Failure::new(
                SpawnBatch::OP,
                "a group item that is no SpawnGroup",
            )))),
        });
        slot(self.body.push(item, since))
    }

    /// Adds a static object, cargo or FARP (built with `ctx.spawn_static(..)`, not awaited).
    pub fn static_object(mut self, object: Cmd<SpawnStatic>) -> Self {
        self.add_static(object);
        self
    }

    /// Adds a static object and returns its typed slot.
    pub fn add_static(&mut self, object: Cmd<SpawnStatic>) -> BatchSlot<SpawnedStatic> {
        let since = object.body.since_api_rev();
        let item = object.body.to_proto(&object.session).and_then(|b| match b {
            Body::SpawnStatic(s) => Ok(v1::SpawnItem {
                spawn: Some(v1::spawn_item::Spawn::StaticObject(s)),
            }),
            _ => Err(Error::Invalid(Box::new(Failure::new(
                SpawnBatch::OP,
                "a static item that is no SpawnStatic",
            )))),
        });
        slot(self.body.push(item, since))
    }

    /// The first failing item stops the batch: the rest are `skipped` and never sent. The
    /// items then go one at a time (the next when the previous answered).
    pub fn stop_on_error(mut self) -> Self {
        self.body.stop_on_error = true;
        self
    }

    /// Items added so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.body.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.body.items.is_empty()
    }
}

/// What one item of a batch did.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BatchItem {
    /// The item's final name (the engine's when it chose one).
    pub name: String,
    /// The group's or static's Eid; `None` when the item failed or was skipped.
    pub eid: Option<Eid>,
    /// Never sent: `stop_on_error`, the batch's deadline or a dropped link stopped the batch
    /// first ([`BatchItem::error`] says which).
    pub skipped: bool,
    /// It ran in an earlier attempt of the same key whose result was lost with the mission
    /// link; the engine found it by name and spawned nothing again.
    pub settled: bool,
    /// `spawn_group` or `spawn_static`.
    pub op: &'static str,
    err: Option<v1::CommandError>,
    failure: Failure,
}

impl BatchItem {
    /// The item spawned (or was found by name).
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.err.is_none() && self.eid.is_some()
    }

    /// Why the item failed or was skipped, as the error its own command would have returned.
    #[must_use]
    pub fn error(&self) -> Option<Error> {
        self.err
            .as_ref()
            .map(|e| Error::from_command_error(self.op, self.failure.clone(), e))
    }
}

/// The result of a batch: one [`BatchItem`] per item, in order.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SpawnedBatch {
    pub items: Vec<BatchItem>,
    airbases: Vec<v1::AirbaseSnap>,
    session: Rc<Session>,
}

impl SpawnedBatch {
    /// The item of `slot` as its own command's result: a [`Group`] or a [`SpawnedStatic`], or
    /// the item's error.
    pub fn get<T: BatchOutput>(&self, slot: &BatchSlot<T>) -> Result<T> {
        T::from_batch(self, slot.index)
    }

    /// How many items spawned (or were found by name).
    #[must_use]
    pub fn ok_count(&self) -> usize {
        self.items.iter().filter(|i| i.is_ok()).count()
    }

    fn item(&self, index: usize) -> Result<(&BatchItem, Eid)> {
        let item = self.items.get(index).ok_or_else(|| {
            Error::Invalid(Box::new(Failure::new(
                SpawnBatch::OP,
                format!("the batch has no item {index}"),
            )))
        })?;
        if let Some(e) = item.error() {
            return Err(e);
        }
        match item.eid {
            Some(eid) => Ok((item, eid)),
            None => Err(Error::Invalid(Box::new(Failure {
                message: "the engine returned no eid for the item".into(),
                ..item.failure.clone()
            }))),
        }
    }
}

mod sealed {
    pub trait Sealed {}
}

/// What a [`BatchSlot`] resolves to: [`Group`] for a group item, [`SpawnedStatic`] for a
/// static.
pub trait BatchOutput: sealed::Sealed + Sized {
    #[doc(hidden)]
    fn from_batch(batch: &SpawnedBatch, index: usize) -> Result<Self>;
}

impl<K: Kind> sealed::Sealed for Group<K> {}

impl<K: Kind> BatchOutput for Group<K> {
    fn from_batch(batch: &SpawnedBatch, index: usize) -> Result<Self> {
        let (item, eid) = batch.item(index)?;
        Ok(Group::new(batch.session.clone(), eid, item.name.clone()))
    }
}

impl sealed::Sealed for SpawnedStatic {}

impl BatchOutput for SpawnedStatic {
    fn from_batch(batch: &SpawnedBatch, index: usize) -> Result<Self> {
        let (item, eid) = batch.item(index)?;
        let airbase = batch
            .airbases
            .iter()
            .find(|a| a.host_eid == eid.to_u64())
            .and_then(|a| Eid::from_u64(a.eid));
        // A kept holder (OnConflict::Keep) reports as the static it is; `kept` is not
        // reported per item.
        Ok(SpawnedStatic {
            eid,
            name: item.name.clone(),
            airbase,
            kept: false,
        })
    }
}

impl CommandBody for SpawnBatch {
    type Output = SpawnedBatch;
    const OP: &'static str = "spawn_batch";
    const KEY_REQUIRED_IN_DEV: bool = true;
    const SINCE_API_REV: u32 = 1;
    fn since_api_rev(&self) -> u32 {
        self.since
    }
    fn target(&self) -> String {
        format!("{} item(s)", self.items.len())
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        if let Some(e) = &self.error {
            return Err(e.clone());
        }
        Ok(Body::SpawnBatch(v1::SpawnBatch {
            items: self.items.clone(),
            stop_on_error: self.stop_on_error,
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<SpawnedBatch> {
        let reply = r.reply.unwrap_or_default();
        if reply.spawns.len() != self.items.len() {
            return Err(Error::Invalid(Box::new(Failure::new(
                Self::OP,
                format!(
                    "the engine answered {} item results for {} items",
                    reply.spawns.len(),
                    self.items.len()
                ),
            ))));
        }
        let items = reply
            .spawns
            .into_iter()
            .zip(&self.items)
            .map(|(x, item)| {
                let op = match &item.spawn {
                    Some(v1::spawn_item::Spawn::StaticObject(_)) => "spawn_static",
                    _ => "spawn_group",
                };
                BatchItem {
                    failure: Failure {
                        op,
                        target: format!("{op} {:?} of the batch", x.name),
                        command_id: r.id.clone(),
                        session: s.id.clone(),
                        ..Failure::default()
                    },
                    name: x.name,
                    eid: Eid::from_u64(x.eid),
                    skipped: x.skipped,
                    settled: x.settled,
                    op,
                    err: x.err,
                }
            })
            .collect();
        Ok(SpawnedBatch {
            items,
            airbases: reply.airbases,
            session: s.clone(),
        })
    }
}
