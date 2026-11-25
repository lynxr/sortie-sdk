//! `ctx.events()`: the session's events, already applied to the replica when you see them (in a
//! `Dead` handler the dead unit no longer counts as alive).
//!
//! ```ignore
//! let mut deaths = ctx.events().of::<v1::Dead>().in_group(&sam);
//! while let Some(dead) = deaths.next().await { ... }
//! ```
//!
//! The generic event form (engine API revision 1): every event decoded from DCS names its
//! `world.event` entry (`v1::Event::dcs_event`), so a logic tells `S_EVENT_DEAD`,
//! `S_EVENT_CRASH` and `S_EVENT_PILOT_DEAD` apart (all `Dead`), or `S_EVENT_TAKEOFF` from
//! `S_EVENT_RUNWAY_TAKEOFF` (both `Takeoff`), with [`TypedEvents::from_dcs`]. Every DCS event
//! without a typed kind is a `v1::Unknown` whose `ctx` (`v1::EventContext`) holds its
//! participants as snaps with Eids and every key DCS sent.
//!
//! ```ignore
//! // Wrecks only: not the pilot's death, which DCS raises while the airframe still flies.
//! let mut wrecks = ctx.events().of::<v1::Dead>()
//!     .from_dcs(v1::DcsEvent::Dead)
//!     .from_dcs(v1::DcsEvent::Crash);
//! // Engine starts of the blue players' aircraft.
//! let mut starts = ctx.events().of::<v1::Unknown>()
//!     .from_dcs(v1::DcsEvent::EngineStartup)
//!     .coalition(Coalition::Blue);
//! ```

use std::rc::Rc;

use sortie_proto::v1::{self, event::Kind};
use sortie_types::{Coalition, Eid};
use sortie_world::{AsGroupKey, GroupKey};
use tokio::sync::broadcast;

use crate::runtime::Session;
use crate::sel::AirbaseSel;

/// A typed event body that can be picked out of the stream with [`Events::of`].
pub trait EventType: Sized + 'static {
    /// The event revision that introduced this kind (`#[sortie::mission(event_rev = ..)]`): a
    /// logic whose event revision in effect (`ctx.event_rev()`) is lower never receives it, and
    /// [`Events::of`] logs a warning saying why.
    const EVENT_REV: u32 = 0;
    fn extract(kind: Kind) -> Option<Self>;
    /// The entity the event is about, used by the filters of [`TypedEvents`]: the initiator
    /// (the unit that was born, died, shot, took off, killed...), `last_known` for
    /// `UnitGone`, the unit for `PlayerEnteredUnit`, `ctx.initiator` for `Unknown`; `None` for
    /// events without one.
    fn subject(&self) -> Option<&v1::EntitySnap>;
    /// The coalition [`TypedEvents::coalition`] matches: the subject's; the new owner for
    /// `AirbaseOwnerChanged`.
    fn coalition(&self) -> Option<i32> {
        self.subject().map(|s| s.coalition)
    }
}

macro_rules! event_types {
    ($($ty:ident $(@ $rev:literal)? => |$e:ident| $subject:expr),* $(,)?) => {
        $(impl EventType for v1::$ty {
            $(const EVENT_REV: u32 = $rev;)?
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
    ResourceEnded @ 1 => |_e| None,
    CapabilityChanged @ 1 => |_e| None,
    // The generic form's participants (an engine of API revision 1).
    Unknown => |e| e.ctx.as_ref().and_then(|c| c.initiator.as_ref()),
);

/// Wave 2 (event revision 2): one per change of an airbase's owner in the engine's world,
/// whatever told the engine (`cause`: DCS's S_EVENT_BASE_CAPTURED, an `AirbaseControl`, a read
/// of DCS). It is about an airbase, not an entity: [`TypedEvents::airbase`] and
/// [`TypedEvents::cause`] filter it, [`TypedEvents::coalition`] matches the new owner.
impl EventType for v1::AirbaseOwnerChanged {
    const EVENT_REV: u32 = 2;
    fn extract(kind: Kind) -> Option<Self> {
        match kind {
            Kind::AirbaseOwnerChanged(x) => Some(x),
            _ => None,
        }
    }
    fn subject(&self) -> Option<&v1::EntitySnap> {
        None
    }
    fn coalition(&self) -> Option<i32> {
        Some(self.owner)
    }
}

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

    /// Only events of body type `E`, e.g. `ctx.events().of::<v1::Dead>()`. A kind of a newer
    /// event revision than the one in effect (`ctx.event_rev()`: the declared one, capped by the
    /// engine's) never arrives; a warning says so (declare `#[sortie::mission(event_rev = N)]`,
    /// or the engine is too old).
    #[must_use]
    pub fn of<E: EventType>(self) -> TypedEvents<E> {
        let (need, have) = (E::EVENT_REV, self.session.event_rev);
        if need > have {
            let kind = std::any::type_name::<E>()
                .rsplit("::")
                .next()
                .unwrap_or("?");
            let declared = self.session.declared.event_rev;
            // Event revision 2 belongs to API revision 3: the engine serves a lower API
            // revision event revision 1 at most (the macro refuses the declaration).
            let api_low = need >= 2 && self.session.declared.api_rev < 3;
            if declared < need || api_low {
                tracing::warn!(
                    "events().of: `{kind}` is an event of event revision {need} and this logic \
                     declares {declared}: nothing will arrive; declare \
                     #[sortie::mission(api_rev = 3, event_rev = {need})] to receive it"
                );
            } else {
                tracing::warn!(
                    "events().of: `{kind}` is an event of event revision {need} and this engine \
                     serves {have}: nothing will arrive (update sortied)"
                );
            }
        }
        TypedEvents {
            inner: self,
            filters: Vec::new(),
            dcs: Vec::new(),
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
    /// `from_dcs`: the `world.event` entries let through (empty = all).
    dcs: Vec<v1::DcsEvent>,
}

impl<E> std::fmt::Debug for TypedEvents<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedEvents")
            .field("type", &std::any::type_name::<E>())
            .field("filters", &self.filters.len())
            .field("dcs", &self.dcs)
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

    /// Only events whose subject is on `coalition` (for `AirbaseOwnerChanged`: whose new owner
    /// is `coalition`).
    #[must_use]
    pub fn coalition(mut self, coalition: Coalition) -> Self {
        self.filters.push(Box::new(move |_, e| {
            e.coalition() == Some(coalition.as_i32())
        }));
        self
    }

    /// Only events for which `pred` holds.
    #[must_use]
    pub fn filter(mut self, pred: impl Fn(&E) -> bool + 'static) -> Self {
        self.filters.push(Box::new(move |_, e| pred(e)));
        self
    }

    /// Only events decoded from the `world.event` entry `entry` (`Event.dcs_event`); call it
    /// again to let several through. An engine older than API revision 1 names no entry, so
    /// against it this matches nothing (`ctx.engine_api_rev()`; a warning is logged).
    #[must_use]
    pub fn from_dcs(mut self, entry: v1::DcsEvent) -> Self {
        if self.inner.session.engine_api_rev == 0 && self.dcs.is_empty() {
            tracing::warn!(
                ?entry,
                "from_dcs: this engine (API revision 0) does not name the world.event entry \
                 of its events; nothing will match"
            );
        }
        self.dcs.push(entry);
        self
    }

    /// The next matching event; `None` once the session ended.
    pub async fn next(&mut self) -> Option<E> {
        self.next_with_dcs_event().await.map(|(e, _)| e)
    }

    /// Like [`TypedEvents::next`], with the `world.event` entry the event was decoded from
    /// (`DcsEvent::Unspecified` for events the engine or the hooks made, and for an entry the
    /// enum lacks: `v1::Event::dcs_name` on the whole event names it).
    pub async fn next_with_dcs_event(&mut self) -> Option<(E, v1::DcsEvent)> {
        loop {
            let ev = self.inner.next().await?;
            let entry = ev.dcs_event();
            if !self.dcs.is_empty() && !self.dcs.contains(&entry) {
                continue;
            }
            if let Some(e) = ev.kind.and_then(E::extract)
                && self.filters.iter().all(|f| f(&self.inner.session, &e))
            {
                return Some((e, entry));
            }
        }
    }
}

impl TypedEvents<v1::AirbaseOwnerChanged> {
    /// Only changes of the owner of `airbase` (an `AirbaseSel`: the airbase's Eid or the Eid of
    /// the FARP static or ship that carries it, its DCS name, its manifest symbol).
    #[must_use]
    pub fn airbase(mut self, airbase: impl Into<AirbaseSel>) -> Self {
        use v1::airbase_sel::Sel;
        let sel = airbase.into().to_proto().sel;
        self.filters.push(Box::new(move |_, e| {
            let Some(a) = e.airbase.as_ref() else {
                return false;
            };
            match &sel {
                Some(Sel::Eid(eid)) => a.eid == *eid || (a.host_eid != 0 && a.host_eid == *eid),
                Some(Sel::Name(name)) => a.name == *name,
                Some(Sel::Ident(ident)) => !ident.is_empty() && a.ident == *ident,
                None => false,
            }
        }));
        self
    }

    /// Only changes that `cause` told the engine about (`v1::OwnerChangeCause::Captured`: DCS's
    /// S_EVENT_BASE_CAPTURED; `Control`: an `AirbaseControl`; `Discovered`: a read of DCS).
    #[must_use]
    pub fn cause(mut self, cause: v1::OwnerChangeCause) -> Self {
        self.filters
            .push(Box::new(move |_, e| e.cause == cause as i32));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The event revision of each kind the SDK knows a revision for (the engine's
    /// `revision::event_rev_of`): `Events::of` warns when it is above the one in effect.
    #[test]
    fn event_kinds_carry_their_event_revision() {
        assert_eq!(<v1::BaseCaptured as EventType>::EVENT_REV, 0);
        assert_eq!(<v1::Dead as EventType>::EVENT_REV, 0);
        assert_eq!(<v1::ResourceEnded as EventType>::EVENT_REV, 1);
        assert_eq!(<v1::CapabilityChanged as EventType>::EVENT_REV, 1);
        assert_eq!(<v1::AirbaseOwnerChanged as EventType>::EVENT_REV, 2);
    }

    /// `coalition()` of an owner change is the new owner (it has no subject entity).
    #[test]
    fn an_owner_change_is_about_its_new_owner() {
        let e = v1::AirbaseOwnerChanged {
            previous: Coalition::Blue.as_i32(),
            owner: Coalition::Red.as_i32(),
            ..Default::default()
        };
        assert!(e.subject().is_none());
        assert_eq!(EventType::coalition(&e), Some(Coalition::Red.as_i32()));
        let kill = v1::Kill {
            initiator: Some(v1::EntitySnap {
                coalition: Coalition::Blue.as_i32(),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(EventType::coalition(&kill), Some(Coalition::Blue.as_i32()));
    }
}
