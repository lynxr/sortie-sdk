//! Airbases (API revision 3): the owner, DCS's own capture and the radio silence of many
//! airbases in one command, and the queries of their parking, runways and control state.
//!
//! * [`AirbaseControl`] (`ctx.airbase_control(bases)`, up to [`MAX_AIRBASES`] = 32):
//!   `.owner(Coalition)`, `.auto_capture(bool)`, `.radio_silent(bool)`, applied to every base in
//!   one DCS frame and read back (a longer list goes in several commands, each its own frame).
//!   A new owner is in `ctx.world()` when the command returns (no waiting for a resync). Each
//!   base has its own result ([`AirbasesControlled`]): what DCS read before and
//!   after, or the error DCS raised for that base; the command fails only when it could not
//!   run (an airbase the world does not hold, one listed twice, nothing to set).
//!
//! ```ignore
//! let out = ctx
//!     .airbase_control([airbases::KUTAISI, airbases::SENAKI])
//!     .owner(Coalition::Red)
//!     .auto_capture(false) // applied first: DCS does not take the base back
//!     .key("front-moved-7")
//!     .await?;
//! for base in &out.bases {
//!     if let Some(e) = base.error() { /* this base only */ }
//! }
//! ```

//!
//! The queries (reads: nothing changes in DCS, no key needed) of one base,
//! `ctx.airbase(sel)` -> [`AirbaseHandle`] with `.parking()`, `.runways()`, `.state()`, and of
//! many, `ctx.airbases_parking(..)`, `ctx.airbases_runways(..)`, `ctx.airbases_state(..)`
//! ([`ReadAirbases`]: each base answered or with its own error):
//!
//! ```ignore
//! let free = ctx.airbase(airbases::KUTAISI).parking().available_only().await?;
//! let helo = free.iter().filter(|s| s.terminal == Some(TerminalType::HelicopterOnly));
//! let owners = ctx.airbases_state([airbases::KUTAISI, airbases::SENAKI]).await?;
//! ```

use std::marker::PhantomData;
use std::panic::Location;
use std::rc::Rc;

use sortie_proto::v1::{self, command::Body};
use sortie_types::{Coalition, Eid, Vec3};

use super::{Cmd, CommandBody};
use crate::runtime::Session;
use crate::sel::AirbaseSel;
use crate::{Error, Failure, Result};

/// The most airbases of one [`AirbaseControl`]: the whole list runs in one DCS frame, about 11
/// DCS calls a base, so a longer list goes in several commands (each its own frame). What a
/// base costs is not measured yet (live check AWC-T20).
pub const MAX_AIRBASES: usize = 32;

/// `ctx.airbase_control(bases)`: set `.owner(..)`, `.auto_capture(..)` and/or
/// `.radio_silent(..)` (at least one), then `.await`.
#[derive(Clone, Debug)]
pub struct AirbaseControl {
    airbases: Vec<AirbaseSel>,
    owner: Option<Coalition>,
    auto_capture: Option<bool>,
    radio_silent: Option<bool>,
}

impl AirbaseControl {
    pub(crate) fn new(airbases: Vec<AirbaseSel>) -> Self {
        Self {
            airbases,
            owner: None,
            auto_capture: None,
            radio_silent: None,
        }
    }
}

impl Cmd<AirbaseControl> {
    /// Gives every base to `coalition` (`Coalition::Neutral` too). With DCS's auto capture on,
    /// ground units near a base may take it back: add `.auto_capture(false)`.
    pub fn owner(mut self, coalition: Coalition) -> Self {
        self.body.owner = Some(coalition);
        self
    }

    /// DCS's own capture of the bases by ground units, on or off. Applied before the owner.
    pub fn auto_capture(mut self, on: bool) -> Self {
        self.body.auto_capture = Some(on);
        self
    }

    /// The bases' ATC silent (`true`) or talking again (`false`).
    pub fn radio_silent(mut self, silent: bool) -> Self {
        self.body.radio_silent = Some(silent);
        self
    }
}

/// The control state of an airbase as DCS answered it; `None` where DCS did not answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct AirbaseState {
    /// `getCoalition()`.
    pub owner: Option<Coalition>,
    /// `autoCaptureIsOn()`.
    pub auto_capture: Option<bool>,
    /// `getRadioSilentMode()`.
    pub radio_silent: Option<bool>,
}

impl AirbaseState {
    fn from_proto(s: &v1::AirbaseControlState) -> Self {
        Self {
            owner: Coalition::from_i32(s.coalition),
            auto_capture: s.auto_capture,
            radio_silent: s.radio_silent,
        }
    }
}

/// What an [`AirbaseControl`] did to one airbase.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct AirbaseControlled {
    /// The world's Eid of the airbase (`None`: the world no longer holds it by that name).
    pub eid: Option<Eid>,
    /// Its DCS name.
    pub name: String,
    /// What DCS read before the change; `None` when the base was not found then.
    pub before: Option<AirbaseState>,
    /// What DCS read after it, in the same frame.
    pub after: Option<AirbaseState>,
    err: Option<v1::CommandError>,
    failure: Failure,
}

impl AirbaseControlled {
    /// Every setting was applied to this base.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.err.is_none()
    }

    /// What DCS raised for this base: `Error::Gone` (DCS has no such airbase now) or
    /// `Error::Lua` (a setter raised; `after` says what DCS reads now).
    #[must_use]
    pub fn error(&self) -> Option<Error> {
        self.err
            .as_ref()
            .map(|e| Error::from_command_error(AirbaseControl::OP, self.failure.clone(), e))
    }
}

/// The result of an [`AirbaseControl`]: one entry per airbase, in the order given.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct AirbasesControlled {
    pub bases: Vec<AirbaseControlled>,
}

impl AirbasesControlled {
    /// How many bases took every setting.
    #[must_use]
    pub fn ok_count(&self) -> usize {
        self.bases.iter().filter(|b| b.is_ok()).count()
    }
}

impl CommandBody for AirbaseControl {
    type Output = AirbasesControlled;
    const OP: &'static str = "airbase_control";
    const SINCE_API_REV: u32 = 3;
    fn target(&self) -> String {
        match self.airbases.as_slice() {
            [one] => format!("airbase {:?}", one.to_proto().sel),
            many => format!("{} airbases", many.len()),
        }
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::AirbaseControl(v1::AirbaseControl {
            airbases: self.airbases.iter().map(AirbaseSel::to_proto).collect(),
            coalition: self.owner.map(Coalition::as_i32),
            auto_capture: self.auto_capture,
            radio_silent: self.radio_silent,
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<AirbasesControlled> {
        let bases = r
            .reply
            .and_then(|x| x.airbase_control)
            .unwrap_or_default()
            .bases
            .into_iter()
            .map(|b| AirbaseControlled {
                failure: Failure {
                    op: Self::OP,
                    target: format!("airbase {:?}", b.name),
                    command_id: r.id.clone(),
                    session: s.id.clone(),
                    ..Failure::default()
                },
                eid: Eid::from_u64(b.eid),
                name: b.name,
                before: b.before.as_ref().map(AirbaseState::from_proto),
                after: b.after.as_ref().map(AirbaseState::from_proto),
                err: b.err,
            })
            .collect();
        Ok(AirbasesControlled { bases })
    }
}

// ----- the airbase queries (API revision 3) ------------------------------------------------------

/// The most airbases of one parking query (a big airfield has hundreds of spots).
pub const MAX_PARKING_BASES: usize = 16;
/// The most airbases of one runway or state query (DCS reads 32 bases a frame).
pub const MAX_READ_BASES: usize = 256;

/// What an airbase query reads: [`Parking`], [`Runways`] or [`ControlState`] (the type
/// parameter of [`ReadAirbases`] and [`ReadAirbase`]).
pub trait AirbasePart: Sized + 'static {
    /// What one base answers.
    type Value;
    /// The operation and capability name.
    const OP: &'static str;
    /// The most airbases of one query.
    const MAX: usize;
    #[doc(hidden)]
    fn kind(airbases: Vec<v1::AirbaseSel>, available_only: bool) -> v1::query::Kind;
    #[doc(hidden)]
    fn value(read: v1::AirbaseRead) -> Self::Value;
}

/// The parking spots of a base (`Airbase.getParking`), in DCS's order.
#[derive(Clone, Copy, Debug)]
pub enum Parking {}

/// The runways of a base (`Airbase.getRunways`); none for a FARP or a ship.
#[derive(Clone, Copy, Debug)]
pub enum Runways {}

/// The owner, auto capture and radio silence of a base as DCS reads them now.
#[derive(Clone, Copy, Debug)]
pub enum ControlState {}

impl AirbasePart for Parking {
    type Value = Vec<ParkingSpot>;
    const OP: &'static str = "airbase_parking";
    const MAX: usize = MAX_PARKING_BASES;
    fn kind(airbases: Vec<v1::AirbaseSel>, available_only: bool) -> v1::query::Kind {
        v1::query::Kind::AirbaseParking(v1::AirbaseParking {
            airbases,
            available_only,
        })
    }
    fn value(read: v1::AirbaseRead) -> Vec<ParkingSpot> {
        read.parking
            .into_iter()
            .map(ParkingSpot::from_proto)
            .collect()
    }
}

impl AirbasePart for Runways {
    type Value = Vec<Runway>;
    const OP: &'static str = "airbase_runways";
    const MAX: usize = MAX_READ_BASES;
    fn kind(airbases: Vec<v1::AirbaseSel>, _: bool) -> v1::query::Kind {
        v1::query::Kind::AirbaseRunways(v1::AirbaseRunways { airbases })
    }
    fn value(read: v1::AirbaseRead) -> Vec<Runway> {
        read.runways.into_iter().map(Runway::from_proto).collect()
    }
}

impl AirbasePart for ControlState {
    type Value = AirbaseState;
    const OP: &'static str = "airbase_state";
    const MAX: usize = MAX_READ_BASES;
    fn kind(airbases: Vec<v1::AirbaseSel>, _: bool) -> v1::query::Kind {
        v1::query::Kind::AirbaseState(v1::AirbaseStates { airbases })
    }
    fn value(read: v1::AirbaseRead) -> AirbaseState {
        read.state
            .as_ref()
            .map(AirbaseState::from_proto)
            .unwrap_or_default()
    }
}

/// A parking spot's `Term_Type`, by the values the DCS scripting documentation lists.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TerminalType {
    /// 16: a spawn point on the runway.
    Runway,
    /// 40: helicopters only.
    HelicopterOnly,
    /// 68: a hardened aircraft shelter.
    Shelter,
    /// 72: open or shelter, airplanes only.
    OpenOrShelter,
    /// 100: a small shelter (Syria and Sinai maps).
    SmallShelter,
    /// 104: open air, generally the larger spots.
    OpenAir,
    /// A number this release has no name for, as `unknown(<n>)`.
    Unknown(String),
}

impl TerminalType {
    fn from_proto(t: i32, unknown: String) -> Option<Self> {
        use v1::TerminalType as T;
        Some(match T::try_from(t).ok()? {
            T::Runway => TerminalType::Runway,
            T::HelicopterOnly => TerminalType::HelicopterOnly,
            T::Shelter => TerminalType::Shelter,
            T::OpenOrShelter => TerminalType::OpenOrShelter,
            T::SmallShelter => TerminalType::SmallShelter,
            T::OpenAir => TerminalType::OpenAir,
            T::Unknown => TerminalType::Unknown(unknown),
            T::Unspecified => return None,
        })
    }
}

/// One parking spot as `Airbase.getParking` answered. A field DCS did not answer is `None`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ParkingSpot {
    /// `Term_Index`: the spot's number at its airbase.
    pub term_index: i32,
    /// `Term_Type`.
    pub terminal: Option<TerminalType>,
    /// `vTerminalPos`.
    pub point: Option<Vec3>,
    /// Not among the free spots DCS lists; `None` when DCS did not answer that.
    pub taken: Option<bool>,
    /// The object DCS names on the spot (`TO_AC`, when DCS answers an object there; the
    /// documentation shows a boolean): the world's Eid when it holds the object.
    pub taken_by: Option<Eid>,
    /// Its DCS name.
    pub taken_by_name: Option<String>,
    /// `fDistToRW`: to the take-off position of the active runway, metres.
    pub dist_to_runway: Option<f64>,
    /// `TO_AC` as DCS answered it, when a boolean (not documented).
    pub to_ac: Option<bool>,
    /// `Term_Index_0` as DCS answered it (not documented).
    pub term_index_0: Option<i32>,
}

impl ParkingSpot {
    fn from_proto(s: v1::ParkingSpot) -> Self {
        Self {
            term_index: s.term_index,
            terminal: TerminalType::from_proto(s.terminal, s.terminal_unknown),
            point: s.point.map(Vec3::from),
            taken: s.taken,
            taken_by: Eid::from_u64(s.taken_by),
            taken_by_name: (!s.taken_by_name.is_empty()).then_some(s.taken_by_name),
            dist_to_runway: s.dist_to_runway,
            to_ac: s.to_ac,
            term_index_0: s.term_index_0,
        }
    }

    /// The spot is free: DCS lists it among the free ones.
    #[must_use]
    pub fn is_free(&self) -> bool {
        self.taken == Some(false)
    }
}

/// One runway as `Airbase.getRunways` answered. A field DCS did not answer is `None`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Runway {
    /// `Name` as text (DCS gives a number, documented as "can be off").
    pub name: String,
    /// `course` as DCS gives it: radians, by the documentation the negative of the true
    /// heading (not measured; see [`Runway::heading_deg`]).
    pub course: Option<f64>,
    /// Metres.
    pub length: Option<f64>,
    /// Metres.
    pub width: Option<f64>,
    /// The centre of the runway.
    pub center: Option<Vec3>,
}

impl Runway {
    fn from_proto(r: v1::Runway) -> Self {
        Self {
            name: r.name,
            course: r.course,
            length: r.length,
            width: r.width,
            center: r.center.map(Vec3::from),
        }
    }

    /// The true heading in degrees, 0..360, read as the DCS documentation says: `-course`
    /// converted from radians. Not measured yet (live check AWC-T23).
    #[must_use]
    pub fn heading_deg(&self) -> Option<f64> {
        self.course.map(|c| (-c).to_degrees().rem_euclid(360.0))
    }
}

/// One base of an airbase query: what DCS answered for it, or its own error.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct AirbaseRead<T> {
    /// The world's Eid of the airbase (`None`: the world holds no live airbase of that name).
    pub eid: Option<Eid>,
    /// Its DCS name (empty for an Eid the world no longer knows).
    pub name: String,
    /// What DCS answered; `None` when this base has an error.
    pub value: Option<T>,
    err: Option<v1::CommandError>,
    failure: Failure,
}

impl<T> AirbaseRead<T> {
    /// DCS answered this base.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.err.is_none()
    }

    /// This base's error: `Error::Gone` (the world holds it as dead, the Eid is stale, or DCS
    /// has no such airbase), `Error::Lua` (a DCS read raised), `Error::Rejected` (the parking
    /// no longer fit into the answer: read that base in another query).
    #[must_use]
    pub fn error(&self) -> Option<Error> {
        self.err
            .as_ref()
            .map(|e| Error::from_command_error(self.failure.op, self.failure.clone(), e))
    }
}

/// The result of a query of many airbases: one entry per airbase, in the order given.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct AirbasesRead<T> {
    pub bases: Vec<AirbaseRead<T>>,
}

impl<T> AirbasesRead<T> {
    /// How many bases DCS answered.
    #[must_use]
    pub fn ok_count(&self) -> usize {
        self.bases.iter().filter(|b| b.is_ok()).count()
    }
}

/// A query of many airbases: `ctx.airbases_parking(..)`, `ctx.airbases_runways(..)`,
/// `ctx.airbases_state(..)` (API revision 3). 1 to 256 airbases (16 for parking), each at most
/// once; `.await` -> [`AirbasesRead`], each base answered or with its own error. The query
/// fails only when it could not run: an empty or too long list, a selector that names no
/// airbase (an unknown manifest symbol, the Eid of a unit), an airbase listed twice.
#[derive(Clone, Debug)]
pub struct ReadAirbases<P> {
    airbases: Vec<AirbaseSel>,
    available_only: bool,
    part: PhantomData<P>,
}

impl<P> ReadAirbases<P> {
    pub(crate) fn new(airbases: Vec<AirbaseSel>) -> Self {
        Self {
            airbases,
            available_only: false,
            part: PhantomData,
        }
    }
}

/// A query of one airbase: `ctx.airbase(sel).parking()`, `.runways()`, `.state()` (API
/// revision 3); `.await` -> what DCS answered, or the base's error (`Error::Gone` when DCS or
/// the world has no such airbase).
#[derive(Clone, Debug)]
pub struct ReadAirbase<P> {
    all: ReadAirbases<P>,
}

impl Cmd<ReadAirbases<Parking>> {
    /// The free spots only (`Airbase.getParking(true)`): every listed spot is free.
    pub fn available_only(mut self) -> Self {
        self.body.available_only = true;
        self
    }
}

impl Cmd<ReadAirbase<Parking>> {
    /// The free spots only (`Airbase.getParking(true)`): every listed spot is free.
    pub fn available_only(mut self) -> Self {
        self.body.all.available_only = true;
        self
    }
}

fn read_failure<P: AirbasePart>(s: &Session, r: &v1::CommandResult, name: &str) -> Failure {
    Failure {
        op: P::OP,
        target: format!("airbase {name:?}"),
        command_id: r.id.clone(),
        session: s.id.clone(),
        ..Failure::default()
    }
}

impl<P: AirbasePart> CommandBody for ReadAirbases<P> {
    type Output = AirbasesRead<P::Value>;
    const OP: &'static str = P::OP;
    const SINCE_API_REV: u32 = 3;
    fn target(&self) -> String {
        match self.airbases.as_slice() {
            [one] => format!("airbase {:?}", one.to_proto().sel),
            many => format!("{} airbases", many.len()),
        }
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::Query(v1::Query {
            kind: Some(P::kind(
                self.airbases.iter().map(AirbaseSel::to_proto).collect(),
                self.available_only,
            )),
        }))
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<AirbasesRead<P::Value>> {
        let read = r
            .reply
            .as_ref()
            .and_then(|x| x.airbase_read.clone())
            .unwrap_or_default();
        let bases = read
            .bases
            .into_iter()
            .map(|b| AirbaseRead {
                failure: read_failure::<P>(s, &r, &b.name),
                eid: Eid::from_u64(b.eid),
                name: b.name.clone(),
                err: b.err.clone(),
                value: b.err.is_none().then(|| P::value(b)),
            })
            .collect();
        Ok(AirbasesRead { bases })
    }
}

impl<P: AirbasePart> CommandBody for ReadAirbase<P> {
    type Output = P::Value;
    const OP: &'static str = P::OP;
    const SINCE_API_REV: u32 = 3;
    fn target(&self) -> String {
        self.all.target()
    }
    fn to_proto(&self, s: &Session) -> Result<Body> {
        self.all.to_proto(s)
    }
    fn decode(self, s: &Rc<Session>, r: v1::CommandResult) -> Result<P::Value> {
        let target = self.all.target();
        let out = self.all.decode(s, r.clone())?;
        let Some(base) = out.bases.into_iter().next() else {
            return Err(Error::with_code(
                v1::ErrorCode::BridgeFault,
                Failure {
                    op: P::OP,
                    target,
                    message: "the engine answered no airbase".into(),
                    command_id: r.id,
                    session: s.id.clone(),
                    ..Failure::default()
                },
            ));
        };
        if let Some(e) = base.error() {
            return Err(e);
        }
        base.value.ok_or_else(|| {
            Error::with_code(
                v1::ErrorCode::BridgeFault,
                Failure {
                    op: P::OP,
                    target,
                    message: "the engine answered the airbase without its part".into(),
                    ..Failure::default()
                },
            )
        })
    }
}

/// One airbase to read (`ctx.airbase(sel)`): `.parking()`, `.runways()`, `.state()`, each a
/// query of API revision 3 (declare `#[sortie::mission(api_rev = 3)]`).
#[derive(Clone, Debug)]
pub struct AirbaseHandle {
    session: Rc<Session>,
    airbase: AirbaseSel,
}

impl AirbaseHandle {
    pub(crate) fn new(session: Rc<Session>, airbase: AirbaseSel) -> Self {
        Self { session, airbase }
    }

    fn read<P: AirbasePart>(&self, at: &'static Location<'static>) -> Cmd<ReadAirbase<P>> {
        Cmd::new(
            self.session.clone(),
            ReadAirbase {
                all: ReadAirbases::new(vec![self.airbase.clone()]),
            },
            at,
        )
    }

    /// Its parking spots (`Airbase.getParking`), with `taken` for each; `.available_only()`
    /// lists the free ones only.
    #[track_caller]
    pub fn parking(&self) -> Cmd<ReadAirbase<Parking>> {
        self.read(Location::caller())
    }

    /// Its runways (`Airbase.getRunways`); none for a FARP or a ship.
    #[track_caller]
    pub fn runways(&self) -> Cmd<ReadAirbase<Runways>> {
        self.read(Location::caller())
    }

    /// Its owner, auto capture and radio silence as DCS reads them now. An owner the world
    /// held otherwise is in `ctx.world()` when the query returns (and one
    /// `AirbaseOwnerChanged{Discovered}`).
    #[track_caller]
    pub fn state(&self) -> Cmd<ReadAirbase<ControlState>> {
        self.read(Location::caller())
    }
}
