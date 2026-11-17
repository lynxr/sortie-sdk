//! `ctx.menus()`: F10 radio menu trees, kept by name (API revision 1).
//!
//! ```ignore
//! let jtac = ctx.menus()
//!     .tree("jtac/viper-1")
//!     .to(&viper_1)                                  // one group, everyone, or a coalition
//!     .under(["JTAC"])
//!     .submenu("Smoke", |s| s.command_arg("smoke-red", "Red", "red").command_arg("smoke-green", "Green", "green"))
//!     .command("check-in", "Check in")
//!     .await?;                                       // MenuTree { change: Created, .. }
//! let mut presses = jtac.presses();
//! while let Some(p) = presses.next().await {
//!     if p.item == "check-in" { /* .. */ }
//! }
//! ```
//!
//! * A tree is a named resource of the session (kind MENU): a put with a name that exists
//!   brings the tree to the new spec. The same spec again costs no DCS call
//!   (`Change::Unchanged`); otherwise the engine removes and adds only the entries that changed
//!   (`Updated`; DCS cannot rename an entry, so a changed entry and the ones after it on its
//!   level are added again, which keeps the order as given). A new audience or `under` puts
//!   every entry again (`Replaced`).
//! * Audiences: everyone, a coalition, or one group. DCS knows a group's menu only while a
//!   player occupies the group: a tree for a group given by name or manifest ident waits
//!   (SUSPENDED, a warning, no DCS call) and the engine installs it when a player takes a slot
//!   of the group and takes it out when the slot empties (a dynamic slot gets a new group id;
//!   the engine follows it). A tree for a group given by Eid ends with
//!   `ResourceEnded{SOURCE_GONE}` when that group is gone.
//! * DCS shows at most 10 entries on one level. A longer level is paged by default
//!   ([`Overflow::Page`]): DCS shows its first 9 entries and a sub-menu "More..." with the rest,
//!   paged the same way (a level of 25: 9, then 9 and "More...", then 7). The page of the
//!   tree's root level is labelled "More... (<tree name>)": that level is shared with other
//!   trees and scenario menu commands, and DCS finds an entry by its label path, so each
//!   tree's page there has its own label. A press of an entry on a page carries the entry's
//!   own id and arg; a page is never pressed. With `.overflow(Overflow::Reject)` such a level
//!   is refused with `Error::Invalid` instead.
//! * Limits (refused with `Error::Invalid`): no two entries with one label on one level, no
//!   entry labelled like a page of its level on a level that pages ("More...", on the root
//!   level also "More... (<tree name>)"), at most 200 nodes (pages do not count), an id on
//!   every command, an `arg` of at most 256 bytes.
//! * A press carries the tree, the command's id and its `arg` ([`TreePress`]); for a group's
//!   tree also the group. Presses of a tree are events of event revision 1.
//! * It lives until removed, until `.ttl(..)` runs out, or until the session ends.
//! * Every menu tree belongs to API revision 1: `#[sortie::mission(api_rev = 1, event_rev = 1)]`.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::panic::Location;
use std::rc::Rc;
use std::time::Duration;

use sortie_proto::v1::{self, command::Body};
use sortie_types::{Coalition, Eid, MenuRef};
use tokio::sync::Notify;

use crate::Result;
use crate::cmd::{Cmd, CommandBody};
use crate::group::AnyGroup;
use crate::map::{Change, Removed, ack};
use crate::runtime::Session;
use crate::sel::Audience;

/// The API revision of menu trees.
pub const SINCE_API_REV: u32 = 1;

/// The entry point: `ctx.menus()`. Cheap to clone.
#[derive(Clone)]
pub struct Menus {
    session: Rc<Session>,
}

impl std::fmt::Debug for Menus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Menus").finish_non_exhaustive()
    }
}

/// What a level of more than 10 entries does (`sortie.v1.MenuOverflow`). DCS shows at most 10
/// entries on one level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Overflow {
    /// The first 9 entries and a sub-menu "More..." holding the rest, paged the same way (on
    /// the tree's root level the page is labelled "More... (<tree name>)").
    #[default]
    Page,
    /// The put is refused (`Error::Invalid`) and nothing changes.
    Reject,
}

impl Overflow {
    /// The wire value.
    #[must_use]
    pub fn to_proto(self) -> v1::MenuOverflow {
        match self {
            Overflow::Page => v1::MenuOverflow::Page,
            Overflow::Reject => v1::MenuOverflow::Reject,
        }
    }
}

/// One level of a tree: commands and sub-menus in the order they are added.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Level {
    nodes: Vec<v1::MenuNode>,
}

impl Level {
    /// A command; `id` comes back in [`TreePress::item`].
    #[must_use]
    pub fn command(self, id: &str, label: &str) -> Self {
        self.command_arg(id, label, "")
    }

    /// A command with a payload that comes back in [`TreePress::arg`] (at most 256 bytes).
    #[must_use]
    pub fn command_arg(mut self, id: &str, label: &str, arg: &str) -> Self {
        self.nodes.push(v1::MenuNode {
            id: id.to_owned(),
            label: label.to_owned(),
            arg: arg.to_owned(),
            ..v1::MenuNode::default()
        });
        self
    }

    /// A command labelled by the manifest menu `menu`; its press also reaches the 0.1.0
    /// handle of `menu` (`MenuPressed.ident`).
    #[must_use]
    pub fn manifest_command<M>(mut self, id: &str, menu: MenuRef<M>) -> Self {
        self.nodes.push(v1::MenuNode {
            id: id.to_owned(),
            ident: menu.ident().to_owned(),
            ..v1::MenuNode::default()
        });
        self
    }

    /// A sub-menu built by `f`; an empty one stays as a command-less sub-menu.
    #[must_use]
    pub fn submenu(mut self, label: &str, f: impl FnOnce(Level) -> Level) -> Self {
        let inner = f(Level::default());
        self.nodes.push(v1::MenuNode {
            label: label.to_owned(),
            keep_empty: inner.nodes.is_empty(),
            children: inner.nodes,
            ..v1::MenuNode::default()
        });
        self
    }

    /// A node built elsewhere.
    #[must_use]
    pub fn node(mut self, node: v1::MenuNode) -> Self {
        self.nodes.push(node);
        self
    }
}

impl Menus {
    pub(crate) fn new(session: Rc<Session>) -> Self {
        Self { session }
    }

    /// A tree named `name` (1..=128 bytes; `/` separates levels for `remove_prefix`). Without
    /// `.to(..)` it is shown to everyone.
    #[track_caller]
    pub fn tree(&self, name: &str) -> Cmd<PutTree> {
        self.put(
            name,
            v1::MenuTree {
                audience: Some(Audience::all().to_proto()),
                ..v1::MenuTree::default()
            },
        )
    }

    /// A put of a whole `sortie.v1.MenuTree` spec (built elsewhere).
    #[track_caller]
    pub fn put(&self, name: &str, spec: v1::MenuTree) -> Cmd<PutTree> {
        Cmd::new(
            self.session.clone(),
            PutTree {
                name: name.to_owned(),
                ttl_s: 0.0,
                spec,
            },
            Location::caller(),
        )
    }

    #[track_caller]
    fn remove_which(&self, which: v1::resource_remove::Which) -> Cmd<RemoveTrees> {
        Cmd::new(
            self.session.clone(),
            RemoveTrees {
                which,
                any_owner: false,
            },
            Location::caller(),
        )
    }

    /// Removes the tree `name` (`Change::Absent` if there is none).
    #[track_caller]
    pub fn remove(&self, name: &str) -> Cmd<RemoveTrees> {
        self.remove_which(v1::resource_remove::Which::Name(name.to_owned()))
    }

    /// Removes this logic's trees whose name starts with `prefix` (end it with `/`).
    #[track_caller]
    pub fn remove_prefix(&self, prefix: &str) -> Cmd<RemoveTrees> {
        self.remove_which(v1::resource_remove::Which::Prefix(prefix.to_owned()))
    }

    /// Removes every menu of this logic (trees, and 0.1.0 menu commands it added).
    #[track_caller]
    pub fn clear(&self) -> Cmd<RemoveTrees> {
        self.remove_which(v1::resource_remove::Which::All(true))
    }

    /// Presses of the tree `name` from now on, whether or not this logic put it.
    #[must_use]
    pub fn presses(&self, name: &str) -> TreePresses {
        TreePresses::new(&self.session, name)
    }
}

/// A menu tree put (`ResourcePut{menu_tree}`).
#[derive(Clone, Debug)]
pub struct PutTree {
    name: String,
    ttl_s: f64,
    spec: v1::MenuTree,
}

impl PutTree {
    /// The wire form.
    #[must_use]
    pub fn to_proto(&self) -> v1::ResourcePut {
        v1::ResourcePut {
            name: self.name.clone(),
            ttl_s: self.ttl_s,
            offline_ttl_s: 0.0,
            spec: Some(v1::resource_put::Spec::MenuTree(self.spec.clone())),
        }
    }

    fn level(&mut self, f: impl FnOnce(Level) -> Level) {
        let nodes = std::mem::take(&mut self.spec.nodes);
        self.spec.nodes = f(Level { nodes }).nodes;
    }
}

impl Cmd<PutTree> {
    /// Who sees it: everyone (`Audience::all()`, the default), a coalition or one group.
    pub fn to(mut self, audience: impl Into<Audience>) -> Self {
        self.body.spec.audience = Some(audience.into().into());
        self
    }

    /// Sub-menu labels from the F10 root the tree hangs under (shared with other menus).
    pub fn under<S: Into<String>>(mut self, path: impl IntoIterator<Item = S>) -> Self {
        self.body.spec.under = path.into_iter().map(Into::into).collect();
        self
    }

    /// A command at the tree's root; see [`Level::command`].
    pub fn command(mut self, id: &str, label: &str) -> Self {
        self.body.level(|l| l.command(id, label));
        self
    }

    /// A command with a payload; see [`Level::command_arg`].
    pub fn command_arg(mut self, id: &str, label: &str, arg: &str) -> Self {
        self.body.level(|l| l.command_arg(id, label, arg));
        self
    }

    /// A command labelled by a manifest menu; see [`Level::manifest_command`].
    pub fn manifest_command<M>(mut self, id: &str, menu: MenuRef<M>) -> Self {
        self.body.level(|l| l.manifest_command(id, menu));
        self
    }

    /// A sub-menu at the tree's root built by `f`.
    pub fn submenu(mut self, label: &str, f: impl FnOnce(Level) -> Level) -> Self {
        self.body.level(|l| l.submenu(label, f));
        self
    }

    /// A node built elsewhere, at the tree's root.
    pub fn node(mut self, node: v1::MenuNode) -> Self {
        self.body.level(|l| l.node(node));
        self
    }

    /// What a level of more than 10 entries does: [`Overflow::Page`] (the default; also when
    /// this is not called) or [`Overflow::Reject`].
    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.body.spec.overflow = overflow.to_proto() as i32;
        self
    }

    /// Ends the tree `ttl` of model time after this put (every put starts it again);
    /// `ResourceEnded{EXPIRED}` follows.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.body.ttl_s = ttl.as_secs_f64();
        self
    }
}

impl CommandBody for PutTree {
    type Output = MenuTree;
    const OP: &'static str = "menu_tree";
    const SINCE_API_REV: u32 = SINCE_API_REV;
    fn target(&self) -> String {
        format!("menu tree `{}`", self.name)
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Put(PutTree::to_proto(self)))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<MenuTree> {
        let (a, change) = ack(Self::OP, r)?;
        Ok(MenuTree {
            presses: TreePresses::new(s, &self.name),
            name: self.name,
            change,
            dcs_calls: a.dcs_calls,
            resource: a.resource.unwrap_or_default(),
            warnings: a.warnings,
        })
    }
}

/// A put menu tree: what the put did, and the presses of the tree from the moment the engine
/// confirmed it.
#[derive(Debug)]
pub struct MenuTree {
    name: String,
    pub change: Change,
    /// DCS calls the put needed.
    pub dcs_calls: u32,
    /// The tree as the mirror now shows it (`phase` SUSPENDED: its group has no player).
    pub resource: v1::ResourceSnap,
    /// Parts of the spec the engine could not honour and what it did instead.
    pub warnings: Vec<String>,
    presses: TreePresses,
}

impl MenuTree {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Nobody occupies the tree's group: nothing is in DCS until a player takes a slot.
    #[must_use]
    pub fn is_suspended(&self) -> bool {
        self.resource.phase == v1::ResourcePhase::Suspended as i32
    }

    /// Presses of this tree since the engine confirmed the put; each press is yielded once,
    /// to whichever stream of this tree asks first.
    #[must_use]
    pub fn presses(&self) -> TreePresses {
        self.presses.clone()
    }
}

/// A press of a tree command.
#[derive(Clone, Debug, PartialEq)]
pub struct TreePress {
    /// The tree (resource name).
    pub tree: String,
    /// The command's id (`MenuNode.id`).
    pub item: String,
    /// The command's `arg` (empty when it has none).
    pub arg: String,
    /// The manifest menu ident of the command (empty when it has none).
    pub ident: String,
    /// The group whose player pressed (trees for a group).
    pub group: Option<AnyGroup>,
    /// The coalition of a coalition tree.
    pub coalition: Option<Coalition>,
    pub model_time: f64,
}

impl TreePress {
    fn from_proto(p: &v1::MenuPressed, model_time: f64) -> Self {
        TreePress {
            tree: p.tree.clone(),
            item: p.item.clone(),
            arg: p.arg.clone(),
            ident: p.ident.clone(),
            group: p.group_eid.and_then(Eid::from_u64).map(|eid| AnyGroup {
                eid: Some(eid),
                name: p.group_name.clone(),
            }),
            coalition: Coalition::from_i32(p.coalition),
            model_time,
        }
    }
}

/// Presses waiting to be read, fed by the runtime (registered in the session).
#[derive(Debug)]
pub(crate) struct TreeInbox {
    tree: String,
    queue: RefCell<VecDeque<(v1::MenuPressed, f64)>>,
    notify: Notify,
}

impl TreeInbox {
    pub(crate) fn offer(&self, p: &v1::MenuPressed, model_time: f64) {
        if !p.tree.is_empty() && p.tree == self.tree {
            self.queue.borrow_mut().push_back((p.clone(), model_time));
            self.notify.notify_one();
        }
    }
}

/// Stream of presses of one tree; ends with the session.
#[derive(Clone)]
pub struct TreePresses {
    session: Rc<Session>,
    inbox: Rc<TreeInbox>,
}

impl std::fmt::Debug for TreePresses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreePresses")
            .field("tree", &self.inbox.tree)
            .finish()
    }
}

impl TreePresses {
    fn new(session: &Rc<Session>, tree: &str) -> Self {
        let inbox = Rc::new(TreeInbox {
            tree: tree.to_owned(),
            queue: RefCell::default(),
            notify: Notify::new(),
        });
        session.register_tree(&inbox);
        Self {
            session: session.clone(),
            inbox,
        }
    }

    /// The next press; `None` once the session ended.
    pub async fn next(&mut self) -> Option<TreePress> {
        let mut end = self.session.end.subscribe();
        loop {
            if let Some((p, t)) = self.inbox.queue.borrow_mut().pop_front() {
                return Some(TreePress::from_proto(&p, t));
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

/// A remove of menus (`ResourceRemove{MENU}`).
#[derive(Clone, Debug)]
pub struct RemoveTrees {
    which: v1::resource_remove::Which,
    any_owner: bool,
}

impl Cmd<RemoveTrees> {
    /// Also remove menus other writers own (they end for their owner with
    /// `ResourceEnded{REMOVED_BY_OTHER}`).
    pub fn any_owner(mut self) -> Self {
        self.body.any_owner = true;
        self
    }
}

impl CommandBody for RemoveTrees {
    type Output = Removed;
    const OP: &'static str = "menu_tree";
    const SINCE_API_REV: u32 = SINCE_API_REV;
    fn target(&self) -> String {
        match &self.which {
            v1::resource_remove::Which::Name(n) => format!("menu tree `{n}`"),
            v1::resource_remove::Which::Prefix(p) => format!("menu trees `{p}*`"),
            v1::resource_remove::Which::All(_) => "every menu".to_owned(),
        }
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Remove(v1::ResourceRemove {
            kind: v1::ResourceKind::Menu as i32,
            which: Some(self.which.clone()),
            any_owner: self.any_owner,
        }))
    }
    fn decode(self, _: &Rc<Session>, r: v1::CommandResult) -> Result<Removed> {
        let (a, change) = ack(Self::OP, r)?;
        Ok(Removed {
            change,
            count: a.count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builder_makes_the_tree_in_order() {
        let l = Level::default()
            .command("a", "Alpha")
            .submenu("Smoke", |s| s.command_arg("red", "Red", "r"))
            .submenu("Empty", |s| s);
        assert_eq!(l.nodes.len(), 3);
        assert_eq!(l.nodes[1].children[0].arg, "r");
        assert!(!l.nodes[1].keep_empty);
        assert!(l.nodes[2].keep_empty && l.nodes[2].children.is_empty());
        let p = TreePress::from_proto(
            &v1::MenuPressed {
                tree: "t".into(),
                item: "red".into(),
                arg: "r".into(),
                group_eid: Some(sortie_proto::eid::pack(5, 1)),
                group_name: "Viper".into(),
                ..v1::MenuPressed::default()
            },
            3.0,
        );
        assert_eq!(p.group.unwrap().name, "Viper");
        assert_eq!((p.item.as_str(), p.arg.as_str()), ("red", "r"));
    }
}
