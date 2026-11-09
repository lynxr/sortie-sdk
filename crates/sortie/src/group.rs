//! Live group handles.

use std::marker::PhantomData;
use std::panic::Location;
use std::rc::Rc;

use sortie_proto::v1;
use sortie_types::{Alarm, Eid, Kind, Roe, kind::Flying};
use sortie_world::{AsGroupKey, GroupKey};

use crate::cmd::{self, Cmd};
use crate::runtime::Session;
use crate::task::Task;

/// An AI option for [`Group::set_option`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiOption {
    Alarm(Alarm),
    Roe(Roe),
}

impl From<Alarm> for AiOption {
    fn from(a: Alarm) -> Self {
        AiOption::Alarm(a)
    }
}

impl From<Roe> for AiOption {
    fn from(r: Roe) -> Self {
        AiOption::Roe(r)
    }
}

/// A live group of kind `K` (`Ground`, `Air`, `Helo`, `Ship`), bound to one generational
/// [`Eid`]: a group respawned under the same name is a different handle.
pub struct Group<K> {
    session: Rc<Session>,
    eid: Eid,
    name: String,
    _k: PhantomData<fn() -> K>,
}

impl<K> Clone for Group<K> {
    fn clone(&self) -> Self {
        Self {
            session: self.session.clone(),
            eid: self.eid,
            name: self.name.clone(),
            _k: PhantomData,
        }
    }
}

impl<K> std::fmt::Debug for Group<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Group")
            .field("eid", &self.eid)
            .field("name", &self.name)
            .finish()
    }
}

impl<K> AsGroupKey for Group<K> {
    fn group_key(&self) -> GroupKey<'_> {
        GroupKey::Eid(self.eid)
    }
}

impl<K> Group<K> {
    pub(crate) fn new(session: Rc<Session>, eid: Eid, name: String) -> Self {
        Self {
            session,
            eid,
            name,
            _k: PhantomData,
        }
    }

    pub(crate) fn selector(&self) -> v1::GroupSel {
        v1::GroupSel {
            sel: Some(v1::group_sel::Sel::Eid(self.eid.to_u64())),
        }
    }

    #[must_use]
    pub fn eid(&self) -> Eid {
        self.eid
    }

    /// DCS group name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Live units right now (from the replica).
    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.session
            .world
            .try_borrow()
            .map_or(0, |w| w.group(&self.eid).alive_count())
    }

    /// `true` while at least one unit of this group (this life of it) is alive.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.alive_count() > 0
    }

    /// This group without its kind.
    #[must_use]
    pub fn any(&self) -> AnyGroup {
        AnyGroup {
            eid: Some(self.eid),
            name: self.name.clone(),
        }
    }
}

impl<K: Kind> Group<K> {
    #[track_caller]
    pub fn set_option(&self, option: impl Into<AiOption>) -> Cmd<cmd::SetOption> {
        let body = cmd::SetOption {
            group: self.selector(),
            group_name: self.name.clone(),
            option: option.into(),
        };
        Cmd::new(self.session.clone(), body, Location::caller())
    }

    #[track_caller]
    pub fn destroy(&self) -> Cmd<cmd::Destroy> {
        Cmd::new(
            self.session.clone(),
            cmd::Destroy {
                group: self.selector(),
                group_name: self.name.clone(),
            },
            Location::caller(),
        )
    }
}

impl<K: Flying> Group<K> {
    /// Pushes `task` on top of the group's task stack (`Controller.pushTask`).
    #[track_caller]
    pub fn push_task(&self, task: impl Into<Task>) -> Cmd<cmd::SetTask> {
        let body = cmd::SetTask {
            group: self.selector(),
            group_name: self.name.clone(),
            push: true,
            task: task.into(),
        };
        Cmd::new(self.session.clone(), body, Location::caller())
    }

    /// Replaces the group's task (`Controller.setTask`).
    #[track_caller]
    pub fn set_task(&self, task: impl Into<Task>) -> Cmd<cmd::SetTask> {
        let body = cmd::SetTask {
            group: self.selector(),
            group_name: self.name.clone(),
            push: false,
            task: task.into(),
        };
        Cmd::new(self.session.clone(), body, Location::caller())
    }
}

/// A group whose kind is not known statically (e.g. the presser of a per-group menu).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnyGroup {
    pub eid: Option<Eid>,
    pub name: String,
}

impl AnyGroup {
    pub(crate) fn selector(&self) -> v1::GroupSel {
        let sel = match self.eid {
            Some(e) => v1::group_sel::Sel::Eid(e.to_u64()),
            None => v1::group_sel::Sel::Name(self.name.clone()),
        };
        v1::GroupSel { sel: Some(sel) }
    }
}

impl AsGroupKey for AnyGroup {
    fn group_key(&self) -> GroupKey<'_> {
        match self.eid {
            Some(e) => GroupKey::Eid(e),
            None => GroupKey::Name(&self.name),
        }
    }
}
