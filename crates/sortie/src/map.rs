//! `ctx.map()`: drawings on the F10 map, kept by name (API revision 1).
//!
//! ```ignore
//! let zone = ctx.map()
//!     .circle("zones/alpha", center, 5_000.0)
//!     .color(Rgba::BLUE)
//!     .fill(Rgba::BLUE.alpha(0.15))
//!     .line_style(LineStyle::Dashed)
//!     .to(Coalition::Blue)
//!     .await?;                                   // Drawn { change: Created, .. }
//! ctx.map().circle("zones/alpha", center, 7_500.0).color(Rgba::BLUE)
//!     .fill(Rgba::BLUE.alpha(0.15)).line_style(LineStyle::Dashed).to(Coalition::Blue)
//!     .await?;                                   // Updated: the radius changed in place
//! ctx.map().remove_prefix("zones/").await?;
//! ```
//!
//! * A drawing is a named resource of the session: a put with a name that exists brings the
//!   drawing to the new spec. The same spec again costs no DCS call (`Change::Unchanged`);
//!   colour, fill, circle radius, the text and font size of a text, the first and the last
//!   point change in place (`Updated`); anything else draws it again with new DCS marks
//!   (`Replaced`). Commands on one name run in the order they were sent.
//! * `.key()` is not needed: a put is its own idempotent form (a key is ignored).
//! * Who sees it: `.to(..)` (repeatable, at most 8); none = everyone. Shapes reach everyone
//!   or a coalition; a panel also one group.
//! * It lives until removed, until `.ttl(..)` runs out, until a player deletes an editable
//!   one, or until the session ends. An end that is not the logic's own remove arrives as the
//!   event `ResourceEnded` (event revision 1); `ctx.world().resources()` lists what exists.
//! * Every drawing belongs to API revision 1: `#[sortie::mission(api_rev = 1)]`.

use std::panic::Location;
use std::rc::Rc;
use std::time::Duration;

use sortie_proto::v1::{self, command::Body};
use sortie_types::MapPoint;

use crate::cmd::{Cmd, CommandBody};
use crate::runtime::Session;
use crate::sel::{Anchor, Audience};
use crate::{Error, Failure, Result};

pub use sortie_proto::v1::LineStyle;

/// The API revision of the registry and its drawings.
pub const SINCE_API_REV: u32 = 1;

/// A colour, each channel 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const RED: Rgba = Rgba::new(1.0, 0.0, 0.0, 1.0);
    pub const BLUE: Rgba = Rgba::new(0.0, 0.0, 1.0, 1.0);
    pub const GREEN: Rgba = Rgba::new(0.0, 1.0, 0.0, 1.0);
    pub const WHITE: Rgba = Rgba::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Rgba = Rgba::new(0.0, 0.0, 0.0, 1.0);

    #[must_use]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// The same colour with opacity `a`.
    #[must_use]
    pub const fn alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    #[must_use]
    pub fn to_proto(self) -> v1::Rgba {
        v1::Rgba {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}

/// What a registry command did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Change {
    Created,
    /// The spec equals the registered one: nothing was sent to DCS (the ttl starts again).
    Unchanged,
    /// Changed in place (the DCS marks kept their ids), or only the ttl or the announce text.
    Updated,
    /// Drawn again with new DCS marks.
    Replaced,
    Removed,
    /// Remove: nothing matched.
    Absent,
}

impl Change {
    fn from_proto(c: i32) -> Option<Self> {
        Some(match v1::ResourceChange::try_from(c).ok()? {
            v1::ResourceChange::Created => Change::Created,
            v1::ResourceChange::Unchanged => Change::Unchanged,
            v1::ResourceChange::Updated => Change::Updated,
            v1::ResourceChange::Replaced => Change::Replaced,
            v1::ResourceChange::Removed => Change::Removed,
            v1::ResourceChange::Absent => Change::Absent,
            v1::ResourceChange::Unspecified => return None,
        })
    }
}

/// The result of a drawing put.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawn {
    pub change: Change,
    /// DCS calls the put needed.
    pub dcs_calls: u32,
    /// The drawing as the mirror now shows it.
    pub resource: v1::ResourceSnap,
    /// Parts of the spec the engine could not honour and what it did instead.
    pub warnings: Vec<String>,
}

/// The result of a remove.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Removed {
    /// `Removed`, or `Absent` when nothing matched.
    pub change: Change,
    /// How many drawings were removed.
    pub count: u32,
}

pub(crate) fn ack(op: &'static str, r: v1::CommandResult) -> Result<(v1::ResourceAck, Change)> {
    let a = r.reply.and_then(|x| x.resource).ok_or_else(|| {
        Error::Invalid(Box::new(Failure::new(
            op,
            "the engine sent no registry result (Reply.resource)",
        )))
    })?;
    let change = Change::from_proto(a.change).ok_or_else(|| {
        Error::Invalid(Box::new(Failure::new(
            op,
            format!("unknown resource change {}", a.change),
        )))
    })?;
    Ok((a, change))
}

/// The entry point: `ctx.map()`. Cheap to clone.
#[derive(Clone)]
pub struct Map {
    session: Rc<Session>,
}

impl std::fmt::Debug for Map {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Map").finish_non_exhaustive()
    }
}

impl Map {
    pub(crate) fn new(session: Rc<Session>) -> Self {
        Self { session }
    }

    #[track_caller]
    fn draw(&self, name: &str, shape: v1::draw::Shape) -> Cmd<Draw> {
        Cmd::new(
            self.session.clone(),
            Draw {
                name: name.to_owned(),
                ttl_s: 0.0,
                spec: v1::Draw {
                    shape: Some(shape),
                    ..v1::Draw::default()
                },
            },
            Location::caller(),
        )
    }

    /// A mark panel with a text box (`markToAll` and friends) at `at`.
    #[track_caller]
    pub fn panel(&self, name: &str, at: impl Into<Anchor>, text: impl Into<String>) -> Cmd<Draw> {
        self.draw(
            name,
            v1::draw::Shape::Panel(v1::MarkPanel {
                at: Some(at.into().into()),
                text: text.into(),
            }),
        )
    }

    /// A straight line.
    #[track_caller]
    pub fn line(&self, name: &str, from: MapPoint, to: MapPoint) -> Cmd<Draw> {
        self.path(name, [from, to])
    }

    /// An open path of 2..=33 points (one DCS line per segment).
    #[track_caller]
    pub fn path(&self, name: &str, points: impl IntoIterator<Item = MapPoint>) -> Cmd<Draw> {
        self.draw(
            name,
            v1::draw::Shape::Path(v1::MapPath {
                points: points.into_iter().map(Into::into).collect(),
                closed: false,
            }),
        )
    }

    /// A closed, fillable polygon of 3..=64 points (4 points: the DCS quad).
    #[track_caller]
    pub fn polygon(&self, name: &str, points: impl IntoIterator<Item = MapPoint>) -> Cmd<Draw> {
        self.draw(
            name,
            v1::draw::Shape::Path(v1::MapPath {
                points: points.into_iter().map(Into::into).collect(),
                closed: true,
            }),
        )
    }

    /// A circle of `radius_m` metres.
    #[track_caller]
    pub fn circle(&self, name: &str, center: MapPoint, radius_m: f64) -> Cmd<Draw> {
        self.draw(
            name,
            v1::draw::Shape::Circle(v1::MapCircle {
                center: Some(center.into()),
                radius_m,
            }),
        )
    }

    /// An axis-aligned rectangle given by two opposite corners.
    #[track_caller]
    pub fn rect(&self, name: &str, a: MapPoint, b: MapPoint) -> Cmd<Draw> {
        self.draw(
            name,
            v1::draw::Shape::Rect(v1::MapRect {
                a: Some(a.into()),
                b: Some(b.into()),
            }),
        )
    }

    /// An arrow pointing at `tip`.
    #[track_caller]
    pub fn arrow(&self, name: &str, tip: MapPoint, tail: MapPoint) -> Cmd<Draw> {
        self.draw(
            name,
            v1::draw::Shape::Arrow(v1::MapArrow {
                tip: Some(tip.into()),
                tail: Some(tail.into()),
            }),
        )
    }

    /// Text on the map (scales with the map zoom); font size with [`Cmd::font_size`].
    #[track_caller]
    pub fn text(&self, name: &str, at: MapPoint, text: impl Into<String>) -> Cmd<Draw> {
        self.draw(
            name,
            v1::draw::Shape::Text(v1::MapText {
                at: Some(at.into()),
                text: text.into(),
                font_size: 0,
            }),
        )
    }

    /// A put of a whole `sortie.v1.Draw` spec (built elsewhere).
    #[track_caller]
    pub fn put(&self, name: &str, spec: v1::Draw) -> Cmd<Draw> {
        Cmd::new(
            self.session.clone(),
            Draw {
                name: name.to_owned(),
                ttl_s: 0.0,
                spec,
            },
            Location::caller(),
        )
    }

    #[track_caller]
    fn remove_which(&self, which: v1::resource_remove::Which) -> Cmd<Remove> {
        Cmd::new(
            self.session.clone(),
            Remove {
                which,
                any_owner: false,
            },
            Location::caller(),
        )
    }

    /// Removes the drawing `name` (`Change::Absent` if there is none). Another writer's
    /// drawing needs [`Cmd::any_owner`].
    #[track_caller]
    pub fn remove(&self, name: &str) -> Cmd<Remove> {
        self.remove_which(v1::resource_remove::Which::Name(name.to_owned()))
    }

    /// Removes this logic's drawings whose name starts with `prefix` (end it with `/`).
    #[track_caller]
    pub fn remove_prefix(&self, prefix: &str) -> Cmd<Remove> {
        self.remove_which(v1::resource_remove::Which::Prefix(prefix.to_owned()))
    }

    /// Removes every drawing of this logic.
    #[track_caller]
    pub fn clear(&self) -> Cmd<Remove> {
        self.remove_which(v1::resource_remove::Which::All(true))
    }
}

/// A drawing put (`ResourcePut{draw}`).
#[derive(Clone, Debug)]
pub struct Draw {
    name: String,
    ttl_s: f64,
    spec: v1::Draw,
}

impl Draw {
    fn style(&mut self) -> &mut v1::MapStyle {
        self.spec.style.get_or_insert_with(v1::MapStyle::default)
    }

    /// The wire form.
    #[must_use]
    pub fn to_proto(&self) -> v1::ResourcePut {
        v1::ResourcePut {
            name: self.name.clone(),
            ttl_s: self.ttl_s,
            offline_ttl_s: 0.0,
            spec: Some(v1::resource_put::Spec::Draw(self.spec.clone())),
        }
    }
}

impl Cmd<Draw> {
    /// Outline / text colour (default: the coalition's colour, white for everyone).
    pub fn color(mut self, c: Rgba) -> Self {
        self.body.style().color = Some(c.to_proto());
        self
    }

    /// Fill of a closed shape, background of a text (default: none).
    pub fn fill(mut self, c: Rgba) -> Self {
        self.body.style().fill = Some(c.to_proto());
        self
    }

    /// Line style (default solid). A change of it draws the drawing again.
    pub fn line_style(mut self, l: LineStyle) -> Self {
        self.body.style().line = l as i32;
        self
    }

    /// Who sees it; repeatable, at most 8. None = everyone.
    pub fn to(mut self, audience: impl Into<Audience>) -> Self {
        self.body.spec.audience.push(audience.into().into());
        self
    }

    /// Players may edit or delete it on the map (an edit marks it drifted, a delete ends it
    /// with `ResourceEnded{PLAYER}`).
    pub fn editable(mut self) -> Self {
        self.body.spec.editable = true;
        self
    }

    /// A message DCS shows the audience when the drawing appears (and when it is drawn
    /// again).
    pub fn announce(mut self, text: impl Into<String>) -> Self {
        self.body.spec.announce = text.into();
        self
    }

    /// The font size of a text (default 16).
    pub fn font_size(mut self, size: u32) -> Self {
        if let Some(v1::draw::Shape::Text(t)) = &mut self.body.spec.shape {
            t.font_size = size;
        }
        self
    }

    /// Ends the drawing `ttl` of model time after this put (every put starts it again);
    /// `ResourceEnded{EXPIRED}` follows.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.body.ttl_s = ttl.as_secs_f64();
        self
    }
}

impl CommandBody for Draw {
    type Output = Drawn;
    const OP: &'static str = "draw";
    const SINCE_API_REV: u32 = SINCE_API_REV;
    fn target(&self) -> String {
        format!("drawing `{}`", self.name)
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Put(Draw::to_proto(self)))
    }
    fn decode(self, _: &Rc<Session>, r: v1::CommandResult) -> Result<Drawn> {
        let (a, change) = ack(Self::OP, r)?;
        Ok(Drawn {
            change,
            dcs_calls: a.dcs_calls,
            resource: a.resource.unwrap_or_default(),
            warnings: a.warnings,
        })
    }
}

/// A remove of drawings (`ResourceRemove{DRAWING}`).
#[derive(Clone, Debug)]
pub struct Remove {
    which: v1::resource_remove::Which,
    any_owner: bool,
}

impl Cmd<Remove> {
    /// Also remove drawings other writers own (they end for their owner with
    /// `ResourceEnded{REMOVED_BY_OTHER}`).
    pub fn any_owner(mut self) -> Self {
        self.body.any_owner = true;
        self
    }
}

impl CommandBody for Remove {
    type Output = Removed;
    const OP: &'static str = "draw";
    const SINCE_API_REV: u32 = SINCE_API_REV;
    fn target(&self) -> String {
        match &self.which {
            v1::resource_remove::Which::Name(n) => format!("drawing `{n}`"),
            v1::resource_remove::Which::Prefix(p) => format!("drawings `{p}*`"),
            v1::resource_remove::Which::All(_) => "every drawing".to_owned(),
        }
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Remove(v1::ResourceRemove {
            kind: v1::ResourceKind::Drawing as i32,
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
