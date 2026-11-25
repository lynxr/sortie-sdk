//! `ctx.dcs()`: DCS functions of the DCS API catalog (`catalog/dcs_api.toml`) with typed
//! wrappers, for what has no typed operation of its own yet.
//!
//! ```ignore
//! let surface = ctx.dcs().land().surface_type(p).await?;           // "ROAD"
//! ctx.dcs().sounds().out_sound("l10n/DEFAULT/alarm.wav").key("alarm").await?;
//! let n = ctx.dcs().warehouse(airbases::KUTAISI).item_count("weapons.missiles.AIM_120C").await?;
//! ```
//!
//! * Every wrapper is generated from the catalog (`cargo xtask catalog-gen`), grouped in
//!   families: `ctx.dcs().land()`, `.timer()`, `.unit(u)`, `.group(g)`, `.airbase(a)`,
//!   `.warehouse(w)`, `.controller(c)`, `.spot(h)`, ... A wrapper is an ordinary [`Cmd`]: it
//!   takes `.key()`, `.timeout()`, `.after()` and is awaited for its typed result.
//! * The engine checks every call against the catalog before anything reaches DCS. A
//!   mutating function is off until the engine owner enables it (`engine.toml [calls]`, or
//!   `sortied --dev`): `Error::Rejected` naming the capability `call:<env>:<path>` until then.
//! * Reads take no key (a stored read would be stale: `Error::Invalid`); a mutating call
//!   repeats its effect unless keyed.
//! * Low level: the mirror does not learn what a mutating call changed until its next sweep
//!   (an airbase owner, a destroyed object). Where a typed operation exists the wrapper is
//!   `#[deprecated]` and names it.
//! * A wrapper belongs to the API revision of its entry, 1 unless its doc names a later one
//!   (the road paths are 3): the logic declares `#[sortie::mission(api_rev = N)]`.
//! * `sortie.*` entries are functions of the Sortie Lua layer over DCS functions, run inside
//!   DCS in the frame of the call: `land().road_path(..)` answers a [`RoadPath`] summary.
//! * The untyped call by string path is behind the cargo feature `dcs-raw`
//!   (`ctx.dcs().call("land.getHeight")`).

use std::marker::PhantomData;
use std::panic::Location;
use std::rc::Rc;

use sortie_proto::LuaValue;
use sortie_proto::v1::{self, command::Body};
use sortie_types::{Coalition, Eid, MapPoint, Vec3};

use crate::cmd::{Cmd, CommandBody};
use crate::runtime::Session;
use crate::sel::{AirbaseSel, ControllerSel, GroupSel, Obj, PlayerSel, UnitSel, WarehouseSel};
use crate::{Error, Failure, Result};

mod generated;
pub use generated::*;

/// The API revision of the universal call.
pub const SINCE_API_REV: u32 = 1;

/// The entry point: `ctx.dcs()`. Cheap to clone.
#[derive(Clone)]
pub struct Dcs {
    session: Rc<Session>,
}

impl std::fmt::Debug for Dcs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dcs").finish_non_exhaustive()
    }
}

impl Dcs {
    pub(crate) fn new(session: Rc<Session>) -> Self {
        Self { session }
    }

    /// A call of the catalog entry `path` (generated code).
    #[track_caller]
    fn typed<T: FromReturn>(
        &self,
        path: &'static str,
        env: v1::Env,
        receiver: Option<v1::DcsRef>,
        args: Vec<v1::DcsArg>,
        since: u32,
    ) -> Cmd<Call<T>> {
        // Trailing nils are left out: the entry's optional arguments.
        let mut args = args;
        while args
            .last()
            .is_some_and(|a| matches!(&a.kind, Some(v1::dcs_arg::Kind::Value(v)) if v.is_nil()))
        {
            args.pop();
        }
        Cmd::new(
            self.session.clone(),
            Call {
                call: v1::DcsCall {
                    path: path.to_owned(),
                    env: env as i32,
                    receiver,
                    args,
                    max_depth: 0,
                },
                since,
                _t: PhantomData,
            },
            Location::caller(),
        )
    }
}

/// One call of the catalog; awaiting it gives `T`.
pub struct Call<T> {
    call: v1::DcsCall,
    since: u32,
    _t: PhantomData<fn() -> T>,
}

impl<T> std::fmt::Debug for Call<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Call")
            .field("path", &self.call.path)
            .finish_non_exhaustive()
    }
}

impl<T> Call<T> {
    /// The wire form.
    #[must_use]
    pub fn to_proto(&self) -> v1::DcsCall {
        self.call.clone()
    }
}

impl<T: FromReturn> Cmd<Call<T>> {
    /// Depth to which DCS tables are copied into the result (default 6, at most 12); deeper
    /// tables come back as [`Value::Cut`].
    pub fn max_depth(mut self, depth: u32) -> Self {
        self.body.call.max_depth = depth;
        self
    }
}

fn decode_failure(msg: impl Into<String>) -> Error {
    Error::Invalid(Box::new(Failure::new("dcs_call", msg)))
}

/// A result without its typed part (`field` of the reply). On a keyed repeat that is the key
/// store's doing: it keeps a result above 64 KiB without its values, and the call was not run
/// again, so there is nothing to decode: [`Error::ResultNotKept`]. Otherwise the engine broke
/// its contract.
fn no_typed_result(replayed: bool, field: &str) -> Error {
    if replayed {
        Error::ResultNotKept(Box::new(Failure::new(
            "dcs_call",
            "the call ran under this key before and was not run again, and the engine kept no \
             values for it: a result above 64 KiB is stored without them",
        )))
    } else {
        decode_failure(format!("the engine sent no typed result ({field})"))
    }
}

impl<T: FromReturn> CommandBody for Call<T> {
    type Output = T;
    const OP: &'static str = "dcs_call";
    const SINCE_API_REV: u32 = SINCE_API_REV;
    fn since_api_rev(&self) -> u32 {
        self.since
    }
    fn target(&self) -> String {
        format!("`{}`", self.call.path)
    }
    fn to_proto(&self, _: &Session) -> Result<Body> {
        Ok(Body::DcsCall(self.call.clone()))
    }
    fn decode(self, _: &Rc<Session>, r: v1::CommandResult) -> Result<T> {
        let replayed = r.replayed;
        let ret = r
            .reply
            .and_then(|x| x.call)
            .ok_or_else(|| no_typed_result(replayed, "Reply.call"))?;
        T::from_return(Return::from_proto(ret))
            .map_err(|m| decode_failure(format!("`{}` returned {m}", self.call.path)))
    }
}

// ----- results --------------------------------------------------------------------------------

/// The class of a returned DCS object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ObjectClass {
    Unit,
    Static,
    Cargo,
    Weapon,
    Scenery,
    Airbase,
    Group,
    Controller,
    Warehouse,
    Spot,
    Other,
}

impl ObjectClass {
    fn from_proto(c: i32) -> Self {
        match v1::ObjectClass::try_from(c).unwrap_or(v1::ObjectClass::Other) {
            v1::ObjectClass::Unit => ObjectClass::Unit,
            v1::ObjectClass::Static => ObjectClass::Static,
            v1::ObjectClass::Cargo => ObjectClass::Cargo,
            v1::ObjectClass::Weapon => ObjectClass::Weapon,
            v1::ObjectClass::Scenery => ObjectClass::Scenery,
            v1::ObjectClass::Airbase => ObjectClass::Airbase,
            v1::ObjectClass::Group => ObjectClass::Group,
            v1::ObjectClass::Controller => ObjectClass::Controller,
            v1::ObjectClass::Warehouse => ObjectClass::Warehouse,
            v1::ObjectClass::Spot => ObjectClass::Spot,
            v1::ObjectClass::Other | v1::ObjectClass::Unspecified => ObjectClass::Other,
        }
    }
}

/// An object the Lua layer keeps for the caller because no selector names it (a spot a call
/// made, a controller or warehouse not derived from a receiver). A handle made by a call lives
/// until the call that ends it (`ctx.dcs().spot(h).destroy()`) or the end of the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Handle(u64);

impl Handle {
    #[must_use]
    pub fn id(self) -> u64 {
        self.0
    }

    /// A handle by its number (as a result reported it).
    #[must_use]
    pub fn from_id(id: u64) -> Self {
        Self(id)
    }
}

/// A DCS object a call returned.
#[derive(Clone, Debug, PartialEq)]
pub struct DcsObject {
    pub class: ObjectClass,
    /// The mirror's Eid; `None` for objects the mirror does not track (and for a light
    /// reference the next sweep observes).
    pub eid: Option<Eid>,
    pub name: String,
    pub type_name: String,
    /// For objects without a selector.
    pub handle: Option<Handle>,
    /// A controller derived from the call's receiver.
    pub controller: Option<ControllerSel>,
    /// A warehouse derived from the call's receiver.
    pub warehouse: Option<WarehouseSel>,
}

impl DcsObject {
    fn from_proto(r: v1::ObjectRef) -> Self {
        Self {
            class: ObjectClass::from_proto(r.object_class),
            eid: Eid::from_u64(r.eid),
            name: r.name,
            type_name: r.type_name,
            handle: (r.handle != 0).then_some(Handle(r.handle)),
            controller: r.controller.map(ControllerSel::from_proto),
            warehouse: r.warehouse.map(WarehouseSel::from_proto),
        }
    }

    /// How a call names this object again (receiver or argument).
    #[must_use]
    pub fn to_ref(&self) -> v1::DcsRef {
        use v1::dcs_ref::Target;
        let target = if let Some(h) = self.handle {
            Target::Handle(h.0)
        } else if let Some(c) = &self.controller {
            Target::Controller(c.to_proto())
        } else if let Some(w) = &self.warehouse {
            Target::Warehouse(w.to_proto())
        } else {
            match (self.class, self.eid) {
                (ObjectClass::Group, Some(e)) => Target::Group(GroupSel::eid(e).to_proto()),
                (ObjectClass::Group, None) => Target::Group(GroupSel::name(&self.name).to_proto()),
                (ObjectClass::Airbase, Some(e)) => Target::Airbase(AirbaseSel::eid(e).to_proto()),
                (ObjectClass::Airbase, None) => {
                    Target::Airbase(AirbaseSel::name(&self.name).to_proto())
                }
                (ObjectClass::Unit, None) => Target::Object(Obj::unit(&self.name).to_proto()),
                (_, Some(e)) => Target::Object(Obj::eid(e).to_proto()),
                (_, None) => Target::Object(Obj::static_object(&self.name).to_proto()),
            }
        };
        v1::DcsRef {
            target: Some(target),
        }
    }
}

impl From<&DcsObject> for Obj {
    /// By Eid when the mirror tracks the object, else by name.
    fn from(o: &DcsObject) -> Self {
        match (o.eid, o.class) {
            (Some(e), _) => Obj::eid(e),
            (None, ObjectClass::Unit) => Obj::unit(&o.name),
            (None, _) => Obj::static_object(&o.name),
        }
    }
}

impl From<&DcsObject> for UnitSel {
    fn from(o: &DcsObject) -> Self {
        o.eid.map_or_else(|| UnitSel::name(&o.name), UnitSel::eid)
    }
}

impl From<&DcsObject> for GroupSel {
    fn from(o: &DcsObject) -> Self {
        o.eid.map_or_else(|| GroupSel::name(&o.name), GroupSel::eid)
    }
}

impl From<&DcsObject> for AirbaseSel {
    fn from(o: &DcsObject) -> Self {
        o.eid
            .map_or_else(|| AirbaseSel::name(&o.name), AirbaseSel::eid)
    }
}

/// One returned value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    String(String),
    /// Ordered pairs (arrays have number keys 1..n).
    Table(Vec<(Value, Value)>),
    Object(DcsObject),
    /// A number of an enum the catalog names, as its key (`"ROAD"`).
    Enum(String),
    /// A table cut by the depth or size limit, with its number of keys.
    Cut(u32),
}

impl Value {
    fn from_proto(v: v1::DcsValue, refs: &[DcsObject]) -> Self {
        use v1::dcs_value::Kind as K;
        match v.kind {
            None | Some(K::Nil(_)) => Value::Nil,
            Some(K::Boolean(b)) => Value::Bool(b),
            Some(K::Number(n)) => Value::Number(n),
            Some(K::String(s)) => Value::String(s),
            Some(K::Table(t)) => Value::Table(
                t.pairs
                    .into_iter()
                    .map(|p| {
                        (
                            Value::from_proto(p.key.unwrap_or_default(), refs),
                            Value::from_proto(p.value.unwrap_or_default(), refs),
                        )
                    })
                    .collect(),
            ),
            Some(K::Ref(i)) => refs
                .get(i as usize)
                .cloned()
                .map_or(Value::Nil, Value::Object),
            Some(K::EnumKey(k)) => Value::Enum(k),
            Some(K::Cut(c)) => Value::Cut(c),
        }
    }

    #[must_use]
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// A string or an enum key.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) | Value::Enum(s) => Some(s),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_object(&self) -> Option<&DcsObject> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    /// The pairs of a table.
    #[must_use]
    pub fn as_table(&self) -> Option<&[(Value, Value)]> {
        match self {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }

    /// The value of a string key of a table.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_table()?
            .iter()
            .find(|(k, _)| matches!(k, Value::String(s) if s == key))
            .map(|(_, v)| v)
    }

    /// The value at a 1-based array index of a table.
    #[must_use]
    pub fn index(&self, i: usize) -> Option<&Value> {
        self.as_table()?
            .iter()
            .find(|(k, _)| matches!(k, Value::Number(n) if *n == i as f64))
            .map(|(_, v)| v)
    }

    /// The array part of a table (keys 1..n in order).
    #[must_use]
    pub fn items(&self) -> Vec<&Value> {
        let mut out = Vec::new();
        while let Some(v) = self.index(out.len() + 1) {
            out.push(v);
        }
        out
    }

    /// The value as JSON (objects as `{"$object": name}`, cuts as `{"$cut": n}`).
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        use serde_json::Value as J;
        match self {
            Value::Nil => J::Null,
            Value::Bool(b) => J::Bool(*b),
            Value::Number(n) => serde_json::Number::from_f64(*n).map_or(J::Null, J::Number),
            Value::String(s) | Value::Enum(s) => J::String(s.clone()),
            Value::Object(o) => serde_json::json!({ "$object": o.name }),
            Value::Cut(n) => serde_json::json!({ "$cut": n }),
            Value::Table(pairs) => {
                let items = self.items();
                if !pairs.is_empty() && items.len() == pairs.len() {
                    J::Array(items.into_iter().map(Value::to_json).collect())
                } else {
                    J::Object(
                        pairs
                            .iter()
                            .map(|(k, v)| {
                                let key = match k {
                                    Value::String(s) => s.clone(),
                                    Value::Number(n) => n.to_string(),
                                    other => format!("{other:?}"),
                                };
                                (key, v.to_json())
                            })
                            .collect(),
                    )
                }
            }
        }
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_owned())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<Vec3> for Value {
    fn from(p: Vec3) -> Self {
        Value::Table(vec![
            ("x".into(), p.x.into()),
            ("y".into(), p.y.into()),
            ("z".into(), p.z.into()),
        ])
    }
}

/// What a call returned.
#[derive(Clone, Debug, PartialEq)]
pub struct Return {
    /// Every return value, in order (nil included).
    pub values: Vec<Value>,
    /// The class of the call (read, mutate, heavy, admin).
    pub class: v1::CallClass,
    /// A depth or size limit cut a table.
    pub truncated: bool,
}

impl Return {
    fn from_proto(r: v1::DcsReturn) -> Self {
        let refs: Vec<DcsObject> = r.refs.into_iter().map(DcsObject::from_proto).collect();
        Self {
            values: r
                .values
                .into_iter()
                .map(|v| Value::from_proto(v, &refs))
                .collect(),
            class: v1::CallClass::try_from(r.call_class).unwrap_or(v1::CallClass::Unspecified),
            truncated: r.truncated,
        }
    }

    /// The first value.
    #[must_use]
    pub fn first(&self) -> &Value {
        self.values.first().unwrap_or(&Value::Nil)
    }
}

/// A result type of a wrapper: built from the call's [`Return`].
pub trait FromReturn: Sized + 'static {
    fn from_return(r: Return) -> std::result::Result<Self, String>;
}

/// One return value of a wrapper.
pub trait FromValue: Sized {
    fn from_value(v: Value) -> std::result::Result<Self, String>;
}

fn kind(v: &Value) -> &'static str {
    match v {
        Value::Nil => "nil",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Table(_) => "a table",
        Value::Object(_) => "an object",
        Value::Enum(_) => "an enum key",
        Value::Cut(_) => "a cut table",
    }
}

impl FromValue for f64 {
    fn from_value(v: Value) -> std::result::Result<Self, String> {
        v.as_f64()
            .ok_or_else(|| format!("{} where a number was expected", kind(&v)))
    }
}

impl FromValue for bool {
    fn from_value(v: Value) -> std::result::Result<Self, String> {
        v.as_bool()
            .ok_or_else(|| format!("{} where a boolean was expected", kind(&v)))
    }
}

impl FromValue for String {
    fn from_value(v: Value) -> std::result::Result<Self, String> {
        match v {
            Value::String(s) | Value::Enum(s) => Ok(s),
            other => Err(format!("{} where a string was expected", kind(&other))),
        }
    }
}

impl FromValue for Value {
    fn from_value(v: Value) -> std::result::Result<Self, String> {
        Ok(v)
    }
}

impl FromValue for Vec3 {
    fn from_value(v: Value) -> std::result::Result<Self, String> {
        let n = |k: &str| v.get(k).and_then(Value::as_f64);
        match (n("x"), n("y"), n("z")) {
            (Some(x), Some(y), Some(z)) => Ok(Vec3::new(x, y, z)),
            _ => Err(format!(
                "{} where a point {{x, y, z}} was expected",
                kind(&v)
            )),
        }
    }
}

impl FromValue for MapPoint {
    fn from_value(v: Value) -> std::result::Result<Self, String> {
        let n = |k: &str| v.get(k).and_then(Value::as_f64);
        match (n("x"), n("y")) {
            (Some(north), Some(east)) => Ok(MapPoint::new(north, east)),
            _ => Err(format!(
                "{} where a map point {{x, y}} was expected",
                kind(&v)
            )),
        }
    }
}

impl FromValue for Coalition {
    fn from_value(v: Value) -> std::result::Result<Self, String> {
        match v.as_str() {
            Some("RED") => Ok(Coalition::Red),
            Some("BLUE") => Ok(Coalition::Blue),
            Some("NEUTRAL" | "NEUTRALS") => Ok(Coalition::Neutral),
            _ => Err(format!("{v:?} where a coalition was expected")),
        }
    }
}

impl FromValue for DcsObject {
    fn from_value(v: Value) -> std::result::Result<Self, String> {
        match v {
            Value::Object(o) => Ok(o),
            other => Err(format!("{} where an object was expected", kind(&other))),
        }
    }
}

impl FromValue for Handle {
    fn from_value(v: Value) -> std::result::Result<Self, String> {
        match v {
            Value::Object(DcsObject {
                handle: Some(h), ..
            }) => Ok(h),
            other => Err(format!("{} where a handle was expected", kind(&other))),
        }
    }
}

impl<T: FromValue> FromValue for Option<T> {
    fn from_value(v: Value) -> std::result::Result<Self, String> {
        if v.is_nil() {
            Ok(None)
        } else {
            T::from_value(v).map(Some)
        }
    }
}

impl<T: FromValue> FromValue for Vec<T> {
    fn from_value(v: Value) -> std::result::Result<Self, String> {
        if v.as_table().is_none() {
            return Err(format!("{} where a list was expected", kind(&v)));
        }
        v.items().into_iter().cloned().map(T::from_value).collect()
    }
}

/// The summary of a road path that the Sortie Lua layer computes inside DCS, in the frame of
/// the call, from what `land.findPathOnRoads` returns: the answer of
/// `ctx.dcs().land().road_path(..)` (catalog entry `sortie.findPathOnRoads`, API revision 3).
/// `count`, `length_m` and the ends are of the whole path; `points` holds at most the cap the
/// call asked for, both ends included (none without a cap).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RoadPath {
    /// The points of the whole path.
    pub count: u32,
    /// Its length along the points, metres.
    pub length_m: f64,
    /// Its first point (`None` for an empty path).
    pub first: Option<MapPoint>,
    /// Its last point.
    pub last: Option<MapPoint>,
    /// How far the first point lies from the start asked, metres.
    pub start_gap_m: Option<f64>,
    /// How far the last point lies from the end asked, metres.
    pub end_gap_m: Option<f64>,
    /// The points kept, in path order: at most the cap, both ends included; the turning points
    /// (`keep` "turns", the default) or points spaced evenly along the path ("even"). Empty
    /// when the call gave no cap.
    pub points: Vec<MapPoint>,
    /// `points` holds fewer points than the path has (so always without a cap, except for an
    /// empty path: `count == 0` is never thinned).
    pub thinned: bool,
}

impl FromValue for RoadPath {
    fn from_value(v: Value) -> std::result::Result<Self, String> {
        if v.as_table().is_none() {
            return Err(format!(
                "{} where a road path summary was expected",
                kind(&v)
            ));
        }
        let take = |key: &str| v.get(key).cloned().unwrap_or(Value::Nil);
        let count = f64::from_value(take("count")).map_err(|m| format!("count: {m}"))?;
        if !(count >= 0.0 && count.fract() == 0.0 && count <= f64::from(u32::MAX)) {
            return Err(format!("count {count} is no number of points"));
        }
        let with = |key: &'static str| move |m: String| format!("{key}: {m}");
        Ok(RoadPath {
            count: count as u32,
            length_m: f64::from_value(take("length")).map_err(with("length"))?,
            first: Option::<MapPoint>::from_value(take("first")).map_err(with("first"))?,
            last: Option::<MapPoint>::from_value(take("last")).map_err(with("last"))?,
            start_gap_m: Option::<f64>::from_value(take("start_gap")).map_err(with("start_gap"))?,
            end_gap_m: Option::<f64>::from_value(take("end_gap")).map_err(with("end_gap"))?,
            points: Vec::<MapPoint>::from_value(take("points")).map_err(with("points"))?,
            thinned: bool::from_value(take("thinned")).map_err(with("thinned"))?,
        })
    }
}

impl FromReturn for () {
    fn from_return(_: Return) -> std::result::Result<Self, String> {
        Ok(())
    }
}

impl FromReturn for Return {
    fn from_return(r: Return) -> std::result::Result<Self, String> {
        Ok(r)
    }
}

macro_rules! single {
    ($($t:ty),*) => {$(
        impl FromReturn for $t {
            fn from_return(r: Return) -> std::result::Result<Self, String> {
                <$t as FromValue>::from_value(r.values.into_iter().next().unwrap_or(Value::Nil))
            }
        }
    )*};
}

single!(
    f64, bool, String, Value, Vec3, MapPoint, Coalition, DcsObject, Handle, RoadPath
);

impl<T: FromValue + 'static> FromReturn for Option<T> {
    fn from_return(r: Return) -> std::result::Result<Self, String> {
        Option::<T>::from_value(r.values.into_iter().next().unwrap_or(Value::Nil))
    }
}

impl<T: FromValue + 'static> FromReturn for Vec<T> {
    fn from_return(r: Return) -> std::result::Result<Self, String> {
        Vec::<T>::from_value(r.values.into_iter().next().unwrap_or(Value::Nil))
    }
}

impl<A: FromValue + 'static, B: FromValue + 'static> FromReturn for (A, B) {
    fn from_return(r: Return) -> std::result::Result<Self, String> {
        let mut it = r.values.into_iter();
        Ok((
            A::from_value(it.next().unwrap_or(Value::Nil))?,
            B::from_value(it.next().unwrap_or(Value::Nil))?,
        ))
    }
}

impl<A: FromValue + 'static, B: FromValue + 'static, C: FromValue + 'static> FromReturn
    for (A, B, C)
{
    fn from_return(r: Return) -> std::result::Result<Self, String> {
        let mut it = r.values.into_iter();
        Ok((
            A::from_value(it.next().unwrap_or(Value::Nil))?,
            B::from_value(it.next().unwrap_or(Value::Nil))?,
            C::from_value(it.next().unwrap_or(Value::Nil))?,
        ))
    }
}

// ----- arguments and receivers (used by the generated wrappers) -----------------------------------

/// A warehouse item as DCS's `Warehouse` functions take it: by its name or by its wsType. One
/// type for the raw wrappers and the typed stock query (`ctx.warehouse(..).stock().items(..)`):
/// defined in [`crate::cmd`], re-exported here, where it has always been.
pub use crate::cmd::WarehouseItem;

/// Builders of [`v1::DcsArg`] (for code that builds calls itself).
pub mod arg {
    use super::{
        AirbaseSel, Coalition, DcsObject, GroupSel, Handle, LuaValue, MapPoint, Obj, PlayerSel,
        UnitSel, Value, Vec3, WarehouseItem, WarehouseSel, v1,
    };
    use v1::dcs_arg::Kind as K;

    fn a(k: K) -> v1::DcsArg {
        v1::DcsArg { kind: Some(k) }
    }

    #[must_use]
    pub fn nil() -> v1::DcsArg {
        a(K::Value(LuaValue::nil()))
    }

    #[must_use]
    pub fn number(n: f64) -> v1::DcsArg {
        a(K::Value(LuaValue::number(n)))
    }

    #[must_use]
    pub fn string(s: impl Into<String>) -> v1::DcsArg {
        a(K::Value(LuaValue::string(s.into())))
    }

    #[must_use]
    pub fn boolean(b: bool) -> v1::DcsArg {
        a(K::Value(LuaValue::boolean(b)))
    }

    /// A warehouse item: its name as a string, its wsType as the table `{a, b, c, d}`.
    #[must_use]
    pub fn item(i: impl Into<WarehouseItem>) -> v1::DcsArg {
        match i.into() {
            WarehouseItem::Name(s) => string(s),
            WarehouseItem::WsType(t) => a(K::Value(LuaValue::table(
                sortie_proto::LuaTable::from_array(t.map(f64::from)),
            ))),
        }
    }

    #[must_use]
    pub fn vec2(p: MapPoint) -> v1::DcsArg {
        a(K::Vec2(p.into()))
    }

    #[must_use]
    pub fn vec3(p: Vec3) -> v1::DcsArg {
        a(K::Vec3(p.into()))
    }

    #[must_use]
    pub fn coalition(c: Coalition) -> v1::DcsArg {
        a(K::Coalition(c.as_i32()))
    }

    /// A key of the enum table `table` (`enum_key("trigger.smokeColor", "Red")`).
    #[must_use]
    pub fn enum_key(table: &str, key: &str) -> v1::DcsArg {
        a(K::EnumKey(format!("{table}.{key}")))
    }

    #[must_use]
    pub fn player(p: PlayerSel) -> v1::DcsArg {
        a(K::Player(p.to_proto()))
    }

    #[must_use]
    pub fn object(r: v1::DcsRef) -> v1::DcsArg {
        a(K::Object(r))
    }

    #[must_use]
    pub fn id_of(r: v1::DcsRef) -> v1::DcsArg {
        a(K::IdOf(r))
    }

    #[must_use]
    pub fn name_of(r: v1::DcsRef) -> v1::DcsArg {
        a(K::NameOf(r))
    }

    /// Data: scalars as values, tables as tables of arguments (objects inside are passed as
    /// objects).
    #[must_use]
    pub fn value(v: impl Into<Value>) -> v1::DcsArg {
        match v.into() {
            Value::Nil | Value::Cut(_) => nil(),
            Value::Bool(b) => boolean(b),
            Value::Number(n) => number(n),
            Value::String(s) | Value::Enum(s) => string(s),
            Value::Object(o) => object(o.to_ref()),
            Value::Table(pairs) => {
                let fields = pairs
                    .into_iter()
                    .filter_map(|(k, v)| {
                        let slot = match k {
                            Value::Number(n)
                                if n >= 1.0 && n.fract() == 0.0 && n <= f64::from(u32::MAX) =>
                            {
                                v1::dcs_field::Slot::Index(n as u32)
                            }
                            Value::Number(n) => v1::dcs_field::Slot::Key(n.to_string()),
                            Value::String(s) => v1::dcs_field::Slot::Key(s),
                            _ => return None,
                        };
                        Some(v1::DcsField {
                            slot: Some(slot),
                            value: Some(value(v)),
                        })
                    })
                    .collect();
                a(K::Table(v1::DcsTable { fields }))
            }
        }
    }

    /// Selector conversions used by the generated wrappers.
    #[must_use]
    pub fn obj(o: impl Into<Obj>) -> v1::DcsRef {
        super::recv::object(o.into())
    }

    #[must_use]
    pub fn unit(u: impl Into<UnitSel>) -> v1::DcsRef {
        super::recv::unit(u.into())
    }

    #[must_use]
    pub fn group(g: impl Into<GroupSel>) -> v1::DcsRef {
        super::recv::group(g.into())
    }

    #[must_use]
    pub fn airbase(a: impl Into<AirbaseSel>) -> v1::DcsRef {
        super::recv::airbase(a.into())
    }

    #[must_use]
    pub fn warehouse(w: impl Into<WarehouseSel>) -> v1::DcsRef {
        super::recv::warehouse(w.into())
    }

    #[must_use]
    pub fn handle(h: Handle) -> v1::DcsRef {
        super::recv::handle(h)
    }

    /// A returned object, passed on.
    #[must_use]
    pub fn returned(o: &DcsObject) -> v1::DcsRef {
        o.to_ref()
    }
}

/// Builders of [`v1::DcsRef`] (receivers).
pub mod recv {
    use super::{AirbaseSel, ControllerSel, GroupSel, Handle, Obj, UnitSel, WarehouseSel, v1};
    use v1::dcs_ref::Target;

    fn r(t: Target) -> v1::DcsRef {
        v1::DcsRef { target: Some(t) }
    }

    #[must_use]
    pub fn object(o: Obj) -> v1::DcsRef {
        r(Target::Object(o.to_proto()))
    }

    /// A unit as an object (`ObjSel` by Eid or unit name).
    #[must_use]
    pub fn unit(u: UnitSel) -> v1::DcsRef {
        let sel = match u.to_proto().sel {
            Some(v1::unit_sel::Sel::Eid(e)) => Some(v1::obj_sel::Sel::Eid(e)),
            Some(v1::unit_sel::Sel::Name(n)) => Some(v1::obj_sel::Sel::UnitName(n)),
            None => None,
        };
        r(Target::Object(v1::ObjSel { sel }))
    }

    #[must_use]
    pub fn group(g: GroupSel) -> v1::DcsRef {
        r(Target::Group(g.to_proto()))
    }

    #[must_use]
    pub fn airbase(a: AirbaseSel) -> v1::DcsRef {
        r(Target::Airbase(a.to_proto()))
    }

    #[must_use]
    pub fn controller(c: ControllerSel) -> v1::DcsRef {
        r(Target::Controller(c.to_proto()))
    }

    #[must_use]
    pub fn warehouse(w: WarehouseSel) -> v1::DcsRef {
        r(Target::Warehouse(w.to_proto()))
    }

    #[must_use]
    pub fn handle(h: Handle) -> v1::DcsRef {
        r(Target::Handle(h.0))
    }
}

// ----- the raw call (feature `dcs-raw`) -------------------------------------------------------------

#[cfg(feature = "dcs-raw")]
mod raw;
#[cfg(feature = "dcs-raw")]
pub use raw::{Batch, BatchReturn, CallResult, RawCall};
