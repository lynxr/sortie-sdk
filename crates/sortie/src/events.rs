//! `ctx.events()`: the session's events, already applied to the replica when you see them (in a
//! `Dead` handler the dead unit no longer counts as alive).
//!
//! ```ignore
//! let mut deaths = ctx.events().of::<v1::Dead>().in_group(&sam);
//! while let Some(dead) = deaths.next().await { ... }
//! ```

use std::rc::Rc;

use sortie_proto::v1::{self, event::Kind};
use sortie_types::{Coalition, Eid};
use sortie_world::{AsGroupKey, GroupKey};
use tokio::sync::broadcast;

use crate::runtime::Session;

/// A typed event body that can be picked out of the stream with [`Events::of`].
pub trait EventType: Sized + 'static {
    fn extract(kind: Kind) -> Option<Self>;
    /// The entity the event is about, used by the filters of [`TypedEvents`]: the initiator
    /// (the unit that was born, died, shot, took off, killed...), `last_known` for
    /// `UnitGone`, the unit for `PlayerEnteredUnit`; `None` for events without one.
    fn subject(&self) -> Option<&v1::EntitySnap>;
}

macro_rules! event_types {
    ($($ty:ident => |$e:ident| $subject:expr),* $(,)?) => {
        $(impl EventType for v1::$ty {
            fn extract(kind: Kind) -> Option<Self> {
                match kind {
                    Kind::$ty(x) => Some(x),
                    _ => None,
                }
            }
            fn subject(&self) -> Option<&v1::EntitySnap> {
                let $e = self;
                $subject
            }
        })*
    };
}

event_types!(
    Birth => |e| e.initiator.as_ref(),
    Dead => |e| e.initiator.as_ref(),
    UnitLost => |e| e.initiator.as_ref(),
    Kill => |e| e.initiator.as_ref(),
    Hit => |e| e.initiator.as_ref(),
    Shot => |e| e.initiator.as_ref(),
    Takeoff => |e| e.initiator.as_ref(),
    Land => |e| e.initiator.as_ref(),
    BaseCaptured => |e| e.initiator.as_ref(),
    Mark => |e| e.initiator.as_ref(),
    PlayerEnteredUnit => |e| e.unit.as_ref(),
    UnitGone => |e| e.last_known.as_ref(),
    MenuPressed => |_e| None,
    PlayerConnected => |_e| None,
    PlayerDisconnected => |_e| None,
    PlayerChangedSlot => |_e| None,
    ChatMessage => |_e| None,
    SimulationPaused => |_e| None,
    SimulationResumed => |_e| None,
    Gap => |_e| None,
    Unknown => |_e| None,
);

/// All events of the session from the moment of subscription.
#[derive(Debug)]
pub struct Events {
    rx: broadcast::Receiver<v1::Event>,
    session: Rc<Session>,
}

impl Events {
    pub(crate) fn new(session: Rc<Session>) -> Self {
        Self {
            rx: session.events.subscribe(),
            session,
        }
    }

    /// The next event; `None` once the session ended. A subscriber that falls more than 1024
    /// events behind skips the oldest ones and logs a warning (the replica stays exact).
    pub async fn next(&mut self) -> Option<v1::Event> {
        let mut end = self.session.end.subscribe();
        loop {
            if end.borrow_and_update().is_some() {
                return None;
            }
            tokio::select! {
                biased;
                r = self.rx.recv() => match r {
                    Ok(ev) => return Some(ev),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "event stream lagged");
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                },
                r = end.changed() => {
                    if r.is_err() {
                        return None;
                    }
                }
            }
        }
    }

    /// Only events of body type `E`, e.g. `ctx.events().of::<v1::Dead>()`.
    #[must_use]
    pub fn of<E: EventType>(self) -> TypedEvents<E> {
        TypedEvents {
            inner: self,
            filters: Vec::new(),
        }
    }
}

/// A group key that outlives the borrowed handle it came from.
#[derive(Clone, Debug)]
enum OwnedKey {
    Eid(Eid),
    Name(String),
    Ident(String),
}

type Filter<E> = Box<dyn Fn(&Session, &E) -> bool>;

/// Events of one body type, optionally filtered.
pub struct TypedEvents<E> {
    inner: Events,
    filters: Vec<Filter<E>>,
}

impl<E> std::fmt::Debug for TypedEvents<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedEvents")
            .field("type", &std::any::type_name::<E>())
            .field("filters", &self.filters.len())
            .finish()
    }
}

impl<E: EventType> TypedEvents<E> {
    /// Only events whose subject belongs to `group` (a `Group<K>` handle, an `AnyGroup`, a
    /// `GroupRef` or a DCS group name). A typed handle matches exactly its own life (Eid); an
    /// ident or a name matches whatever group carries it when the event arrives.
    #[must_use]
    pub fn in_group<G: AsGroupKey + ?Sized>(mut self, group: &G) -> Self {
        let key = match group.group_key() {
            GroupKey::Eid(e) => OwnedKey::Eid(e),
            GroupKey::Name(n) => OwnedKey::Name(n.to_owned()),
            GroupKey::Ident(i) => OwnedKey::Ident(i.to_owned()),
        };
        self.filters.push(Box::new(move |session, e| {
            let Some(s) = e.subject() else { return false };
            match &key {
                OwnedKey::Eid(eid) => s.group_eid == Some(eid.to_u64()),
                OwnedKey::Name(name) => s.group_name == *name,
                OwnedKey::Ident(ident) => session
                    .world
                    .try_borrow()
                    .ok()
                    .and_then(|w| w.group_entry(GroupKey::Ident(ident)).map(|g| g.eid))
                    .is_some_and(|eid| s.group_eid == Some(eid.to_u64())),
            }
        }));
        self
    }

    /// Only events whose subject is on `coalition`.
    #[must_use]
    pub fn coalition(mut self, coalition: Coalition) -> Self {
        self.filters.push(Box::new(move |_, e| {
            e.subject()
                .is_some_and(|s| s.coalition == coalition.as_i32())
        }));
        self
    }

    /// Only events for which `pred` holds.
    #[must_use]
    pub fn filter(mut self, pred: impl Fn(&E) -> bool + 'static) -> Self {
        self.filters.push(Box::new(move |_, e| pred(e)));
        self
    }

    /// The next matching event; `None` once the session ended.
    pub async fn next(&mut self) -> Option<E> {
        loop {
            let ev = self.inner.next().await?;
            if let Some(e) = ev.kind.and_then(E::extract)
                && self.filters.iter().all(|f| f(&self.inner.session, &e))
            {
                return Some(e);
            }
        }
    }
}
