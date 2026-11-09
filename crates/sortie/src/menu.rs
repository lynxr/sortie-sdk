//! Radio (F10) menus. `ctx.menu(coalition)` is one menu for the whole coalition (a press does
//! not say who pressed); `.per_group()` installs the command for every group of the coalition
//! (`missionCommands.addCommandForGroup` on BIRTH), so each press carries the group.
//!
//! A [`MenuCommand`] collects its presses from the moment the engine confirmed it, so a press
//! that arrives before the first [`MenuCommand::presses`] call is not lost. Each press is
//! yielded once, to whichever [`Presses`] stream asks first.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::panic::Location;
use std::rc::Rc;

use sortie_proto::v1;
use sortie_types::{Coalition, Eid, MenuRef};
use tokio::sync::Notify;

use crate::cmd::{AddMenu, Cmd, RemoveMenu};
use crate::group::AnyGroup;
use crate::runtime::Session;

mod sealed {
    pub trait Sealed {}
}

/// Menu scope marker: [`CoalitionWide`] or [`PerGroup`].
pub trait MenuScope: sealed::Sealed + 'static {
    /// What a press yields.
    type Press;
    const SCOPE: v1::MenuScope;
    fn press(ev: &v1::MenuPressed, model_time: f64) -> Self::Press;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CoalitionWide;

#[derive(Clone, Copy, Debug, Default)]
pub struct PerGroup;

impl sealed::Sealed for CoalitionWide {}
impl sealed::Sealed for PerGroup {}

/// A press of a coalition-wide menu command.
#[derive(Clone, Debug, PartialEq)]
pub struct CoalitionPress {
    pub coalition: Option<Coalition>,
    pub model_time: f64,
}

/// A press of a per-group menu command.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupPress {
    /// The group whose member pressed (use it as a message target: `ctx.message(p.group)`).
    pub group: AnyGroup,
    pub coalition: Option<Coalition>,
    pub model_time: f64,
}

impl MenuScope for CoalitionWide {
    type Press = CoalitionPress;
    const SCOPE: v1::MenuScope = v1::MenuScope::Coalition;
    fn press(ev: &v1::MenuPressed, model_time: f64) -> CoalitionPress {
        CoalitionPress {
            coalition: Coalition::from_i32(ev.coalition),
            model_time,
        }
    }
}

impl MenuScope for PerGroup {
    type Press = GroupPress;
    const SCOPE: v1::MenuScope = v1::MenuScope::PerGroup;
    fn press(ev: &v1::MenuPressed, model_time: f64) -> GroupPress {
        GroupPress {
            group: AnyGroup {
                eid: ev.group_eid.and_then(Eid::from_u64),
                name: ev.group_name.clone(),
            },
            coalition: Coalition::from_i32(ev.coalition),
            model_time,
        }
    }
}

/// `ctx.menu(coalition)` builder.
pub struct MenuBuilder<M, S> {
    session: Rc<Session>,
    coalition: Coalition,
    _p: PhantomData<fn() -> (M, S)>,
}

impl<M, S> std::fmt::Debug for MenuBuilder<M, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuBuilder")
            .field("coalition", &self.coalition)
            .finish()
    }
}

impl<M> MenuBuilder<M, CoalitionWide> {
    pub(crate) fn new(session: Rc<Session>, coalition: Coalition) -> Self {
        Self {
            session,
            coalition,
            _p: PhantomData,
        }
    }

    /// Install the commands per group, so presses carry the group.
    #[must_use]
    pub fn per_group(self) -> MenuBuilder<M, PerGroup> {
        MenuBuilder {
            session: self.session,
            coalition: self.coalition,
            _p: PhantomData,
        }
    }
}

impl<M, S: MenuScope> MenuBuilder<M, S> {
    /// Adds the manifest menu command `menu` (label and path come from the manifest).
    #[track_caller]
    pub fn command(self, menu: MenuRef<M>) -> Cmd<AddMenu<S>> {
        let body = AddMenu {
            coalition: self.coalition,
            ident: menu.ident(),
            _s: PhantomData,
        };
        Cmd::new(self.session, body, Location::caller())
    }
}

/// Presses waiting to be read, fed by the runtime (registered in the session).
#[derive(Debug)]
pub(crate) struct MenuInbox {
    ident: &'static str,
    coalition: Coalition,
    queue: RefCell<VecDeque<(v1::MenuPressed, f64)>>,
    notify: Notify,
}

impl MenuInbox {
    pub(crate) fn offer(&self, p: &v1::MenuPressed, model_time: f64) {
        if p.ident == self.ident
            && Coalition::from_i32(p.coalition).is_none_or(|c| c == self.coalition)
        {
            self.queue.borrow_mut().push_back((p.clone(), model_time));
            self.notify.notify_one();
        }
    }
}

/// An installed menu command.
pub struct MenuCommand<S> {
    session: Rc<Session>,
    coalition: Coalition,
    ident: &'static str,
    inbox: Rc<MenuInbox>,
    _s: PhantomData<fn() -> S>,
}

/// The installed menu command `ctx.menu(..).command(..).await` resolves to.
pub type Menu<S> = MenuCommand<S>;

impl<S> std::fmt::Debug for MenuCommand<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuCommand")
            .field("ident", &self.ident)
            .field("coalition", &self.coalition)
            .finish()
    }
}

impl<S: MenuScope> MenuCommand<S> {
    pub(crate) fn new(session: Rc<Session>, coalition: Coalition, ident: &'static str) -> Self {
        let inbox = Rc::new(MenuInbox {
            ident,
            coalition,
            queue: RefCell::default(),
            notify: Notify::new(),
        });
        session.register_menu(&inbox);
        Self {
            session,
            coalition,
            ident,
            inbox,
            _s: PhantomData,
        }
    }

    #[must_use]
    pub fn ident(&self) -> &'static str {
        self.ident
    }

    /// Presses of this command (since it was installed); ends with the session.
    #[must_use]
    pub fn presses(&self) -> Presses<S> {
        Presses {
            session: self.session.clone(),
            inbox: self.inbox.clone(),
            _s: PhantomData,
        }
    }

    #[track_caller]
    pub fn remove(&self) -> Cmd<RemoveMenu> {
        let body = RemoveMenu {
            scope: S::SCOPE,
            coalition: self.coalition,
            ident: self.ident,
        };
        Cmd::new(self.session.clone(), body, Location::caller())
    }
}

/// Stream of presses of one menu command.
pub struct Presses<S> {
    session: Rc<Session>,
    inbox: Rc<MenuInbox>,
    _s: PhantomData<fn() -> S>,
}

impl<S> std::fmt::Debug for Presses<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Presses")
            .field("ident", &self.inbox.ident)
            .finish()
    }
}

impl<S: MenuScope> Presses<S> {
    /// The next press; `None` once the session ended.
    pub async fn next(&mut self) -> Option<S::Press> {
        let mut end = self.session.end.subscribe();
        loop {
            if let Some((p, t)) = self.inbox.queue.borrow_mut().pop_front() {
                return Some(S::press(&p, t));
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
