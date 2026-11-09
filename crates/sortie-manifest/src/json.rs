//! `manifest.json`: the proto3 JSON form of a [`Manifest`].
//!
//! * [`manifest_to_json`] writes exactly what the generator writes with Python protobuf:
//!   `json_format.MessageToJson(m, preserving_proto_field_name=True, indent=2) + "\n"` — fields
//!   in field-number order, default values omitted, enums by name, non-ASCII escaped, floats
//!   in Python `repr` style (`500.0`, `1e-05`, `1e+16`), NaN/Infinity as strings. Map entries
//!   (`tools.extra`) are written sorted by key.
//! * [`manifest_from_json`] reads any proto3 JSON encoding of a manifest: proto field names or
//!   lowerCamelCase, enums by name or number, integers as numbers or strings, `null` as the
//!   default. Unknown fields are an error that names the path.
//!
//! The JSON is for humans and tests; the engine and the digest only ever use `manifest.pb`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Map, Value};
use sortie_proto::manifest::v1 as pb;
use sortie_proto::v1;

use crate::Manifest;

/// A manifest JSON document that does not decode.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("manifest JSON: {path}: {message}")]
pub struct JsonError {
    /// Dotted path of the offending value, e.g. `groups[0].units[2].alt_m`.
    pub path: String,
    pub message: String,
}

fn err(path: &str, message: impl Into<String>) -> JsonError {
    JsonError {
        path: if path.is_empty() {
            "<root>".into()
        } else {
            path.to_owned()
        },
        message: message.into(),
    }
}

// ----- decoding --------------------------------------------------------------------------------

/// Parses a manifest from proto3 JSON (see the module docs).
pub fn manifest_from_json(text: &str) -> Result<Manifest, JsonError> {
    let value: Value =
        serde_json::from_str(text).map_err(|e| err("", format!("not valid JSON: {e}")))?;
    manifest(&value, "")
}

struct Obj<'a> {
    path: String,
    map: &'a Map<String, Value>,
    used: Vec<&'a str>,
}

impl<'a> Obj<'a> {
    fn new(v: &'a Value, path: &str) -> Result<Self, JsonError> {
        match v {
            Value::Object(map) => Ok(Self {
                path: path.to_owned(),
                map,
                used: Vec::new(),
            }),
            other => Err(err(
                path,
                format!("expected an object, got {}", kind(other)),
            )),
        }
    }

    fn sub(&self, name: &str) -> String {
        if self.path.is_empty() {
            name.to_owned()
        } else {
            format!("{}.{name}", self.path)
        }
    }

    /// The value under the proto name or the lowerCamelCase JSON name (`null` = absent).
    fn get(&mut self, name: &'static str) -> Result<Option<&'a Value>, JsonError> {
        let camel = camel_case(name);
        let a = self.map.get_key_value(name);
        let b = (camel != name)
            .then(|| self.map.get_key_value(camel.as_str()))
            .flatten();
        if a.is_some() && b.is_some() {
            return Err(err(
                &self.sub(name),
                format!("both {name:?} and {camel:?} are present"),
            ));
        }
        Ok(match a.or(b) {
            Some((k, v)) => {
                self.used.push(k.as_str());
                (!v.is_null()).then_some(v)
            }
            None => None,
        })
    }

    fn string(&mut self, name: &'static str) -> Result<String, JsonError> {
        match self.get(name)? {
            None => Ok(String::new()),
            Some(Value::String(s)) => Ok(s.clone()),
            Some(other) => Err(err(
                &self.sub(name),
                format!("expected a string, got {}", kind(other)),
            )),
        }
    }

    fn boolean(&mut self, name: &'static str) -> Result<bool, JsonError> {
        match self.get(name)? {
            None => Ok(false),
            Some(Value::Bool(b)) => Ok(*b),
            Some(other) => Err(err(
                &self.sub(name),
                format!("expected true or false, got {}", kind(other)),
            )),
        }
    }

    fn integer<T: TryFrom<i64> + Default>(&mut self, name: &'static str) -> Result<T, JsonError> {
        let path = self.sub(name);
        let Some(v) = self.get(name)? else {
            return Ok(T::default());
        };
        let n: i64 = match v {
            Value::Number(n) => match (n.as_i64(), n.as_f64()) {
                (Some(i), _) => i,
                (None, Some(f)) if f.fract() == 0.0 && f.abs() < 9.0e15 => f as i64,
                _ => return Err(err(&path, format!("{n} is not an integer"))),
            },
            Value::String(s) => s
                .trim()
                .parse()
                .map_err(|_| err(&path, format!("{s:?} is not an integer")))?,
            other => {
                return Err(err(
                    &path,
                    format!("expected an integer, got {}", kind(other)),
                ));
            }
        };
        T::try_from(n).map_err(|_| err(&path, format!("{n} is out of range")))
    }

    fn double(&mut self, name: &'static str) -> Result<f64, JsonError> {
        let path = self.sub(name);
        match self.get(name)? {
            None => Ok(0.0),
            Some(Value::Number(n)) => n
                .as_f64()
                .ok_or_else(|| err(&path, format!("{n} is not a number"))),
            Some(Value::String(s)) => match s.as_str() {
                "NaN" => Ok(f64::NAN),
                "Infinity" => Ok(f64::INFINITY),
                "-Infinity" => Ok(f64::NEG_INFINITY),
                other => other
                    .parse()
                    .map_err(|_| err(&path, format!("{other:?} is not a number"))),
            },
            Some(other) => Err(err(
                &path,
                format!("expected a number, got {}", kind(other)),
            )),
        }
    }

    fn enumeration(
        &mut self,
        name: &'static str,
        from_name: fn(&str) -> Option<i32>,
    ) -> Result<i32, JsonError> {
        let path = self.sub(name);
        match self.get(name)? {
            None => Ok(0),
            Some(Value::String(s)) => {
                from_name(s).ok_or_else(|| err(&path, format!("unknown enum value {s:?}")))
            }
            Some(Value::Number(n)) => n
                .as_i64()
                .and_then(|i| i32::try_from(i).ok())
                .ok_or_else(|| err(&path, format!("{n} is not an enum number"))),
            Some(other) => Err(err(
                &path,
                format!("expected an enum name, got {}", kind(other)),
            )),
        }
    }

    fn message<T>(
        &mut self,
        name: &'static str,
        f: fn(&Value, &str) -> Result<T, JsonError>,
    ) -> Result<Option<T>, JsonError> {
        let path = self.sub(name);
        self.get(name)?.map(|v| f(v, &path)).transpose()
    }

    fn list<T>(
        &mut self,
        name: &'static str,
        f: impl Fn(&Value, &str) -> Result<T, JsonError>,
    ) -> Result<Vec<T>, JsonError> {
        let path = self.sub(name);
        match self.get(name)? {
            None => Ok(Vec::new()),
            Some(Value::Array(items)) => items
                .iter()
                .enumerate()
                .map(|(i, v)| f(v, &format!("{path}[{i}]")))
                .collect(),
            Some(other) => Err(err(
                &path,
                format!("expected an array, got {}", kind(other)),
            )),
        }
    }

    fn strings(&mut self, name: &'static str) -> Result<Vec<String>, JsonError> {
        self.list(name, |v, p| match v {
            Value::String(s) => Ok(s.clone()),
            other => Err(err(p, format!("expected a string, got {}", kind(other)))),
        })
    }

    fn finish(self) -> Result<(), JsonError> {
        match self.map.keys().find(|k| !self.used.contains(&k.as_str())) {
            Some(k) => Err(err(&self.sub(k), "unknown field")),
            None => Ok(()),
        }
    }
}

fn kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

fn camel_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut upper = false;
    for c in name.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn category(s: &str) -> Option<i32> {
    v1::Category::from_str_name(s).map(|v| v as i32)
}

fn coalition(s: &str) -> Option<i32> {
    v1::Coalition::from_str_name(s).map(|v| v as i32)
}

fn start_type(s: &str) -> Option<i32> {
    v1::StartType::from_str_name(s).map(|v| v as i32)
}

fn map_point(v: &Value, path: &str) -> Result<v1::MapPoint, JsonError> {
    let mut o = Obj::new(v, path)?;
    let p = v1::MapPoint {
        north: o.double("north")?,
        east: o.double("east")?,
    };
    o.finish()?;
    Ok(p)
}

fn lat_lon(v: &Value, path: &str) -> Result<v1::LatLon, JsonError> {
    let mut o = Obj::new(v, path)?;
    let p = v1::LatLon {
        lat: o.double("lat")?,
        lon: o.double("lon")?,
    };
    o.finish()?;
    Ok(p)
}

fn unit(v: &Value, path: &str) -> Result<pb::Unit, JsonError> {
    let mut o = Obj::new(v, path)?;
    let u = pb::Unit {
        name: o.string("name")?,
        type_name: o.string("type_name")?,
        position: o.message("position", map_point)?,
        alt_m: o.double("alt_m")?,
        heading_rad: o.double("heading_rad")?,
        unit_id: o.integer("unit_id")?,
        latlon: o.message("latlon", lat_lon)?,
        skill: o.string("skill")?,
    };
    o.finish()?;
    Ok(u)
}

fn group(v: &Value, path: &str) -> Result<pb::Group, JsonError> {
    let mut o = Obj::new(v, path)?;
    let g = pb::Group {
        ident: o.string("ident")?,
        name: o.string("name")?,
        category: o.enumeration("category", category)?,
        coalition: o.enumeration("coalition", coalition)?,
        country: o.string("country")?,
        country_id: o.integer("country_id")?,
        roles: o.strings("roles")?,
        units: o.list("units", unit)?,
        group_id: o.integer("group_id")?,
        late_activation: o.boolean("late_activation")?,
    };
    o.finish()?;
    Ok(g)
}

fn zone(v: &Value, path: &str) -> Result<pb::Zone, JsonError> {
    let mut o = Obj::new(v, path)?;
    let ident = o.string("ident")?;
    let name = o.string("name")?;
    let circle = o.message("circle", |v, p| {
        let mut c = Obj::new(v, p)?;
        let circle = pb::Circle {
            center: c.message("center", map_point)?,
            radius_m: c.double("radius_m")?,
        };
        c.finish()?;
        Ok(circle)
    })?;
    let polygon = o.message("polygon", |v, p| {
        let mut c = Obj::new(v, p)?;
        let polygon = pb::Polygon {
            points: c.list("points", map_point)?,
        };
        c.finish()?;
        Ok(polygon)
    })?;
    let shape = match (circle, polygon) {
        (Some(_), Some(_)) => {
            return Err(err(
                path,
                "a zone has either a circle or a polygon, not both",
            ));
        }
        (Some(c), None) => Some(pb::zone::Shape::Circle(c)),
        (None, Some(p)) => Some(pb::zone::Shape::Polygon(p)),
        (None, None) => None,
    };
    o.finish()?;
    Ok(pb::Zone { ident, name, shape })
}

fn template(v: &Value, path: &str) -> Result<pb::Template, JsonError> {
    let mut o = Obj::new(v, path)?;
    let t = pb::Template {
        ident: o.string("ident")?,
        name: o.string("name")?,
        category: o.enumeration("category", category)?,
        coalition: o.enumeration("coalition", coalition)?,
        country: o.string("country")?,
        country_id: o.integer("country_id")?,
        roles: o.strings("roles")?,
        units: o.list("units", unit)?,
        task: o.string("task")?,
        dcs_group_json: o.string("dcs_group_json")?,
    };
    o.finish()?;
    Ok(t)
}

fn flag(v: &Value, path: &str) -> Result<pb::Flag, JsonError> {
    let mut o = Obj::new(v, path)?;
    let f = pb::Flag {
        ident: o.string("ident")?,
        name: o.string("name")?,
        initial: o.integer("initial")?,
    };
    o.finish()?;
    Ok(f)
}

fn airbase(v: &Value, path: &str) -> Result<pb::Airbase, JsonError> {
    let mut o = Obj::new(v, path)?;
    let a = pb::Airbase {
        ident: o.string("ident")?,
        name: o.string("name")?,
        position: o.message("position", map_point)?,
        airbase_id: o.integer("airbase_id")?,
        coalition: o.enumeration("coalition", coalition)?,
    };
    o.finish()?;
    Ok(a)
}

fn menu(v: &Value, path: &str) -> Result<pb::Menu, JsonError> {
    let mut o = Obj::new(v, path)?;
    let m = pb::Menu {
        ident: o.string("ident")?,
        label: o.string("label")?,
        path: o.strings("path")?,
    };
    o.finish()?;
    Ok(m)
}

fn player_flight(v: &Value, path: &str) -> Result<pb::PlayerFlight, JsonError> {
    let mut o = Obj::new(v, path)?;
    let p = pb::PlayerFlight {
        name: o.string("name")?,
        type_name: o.string("type_name")?,
        size: o.integer("size")?,
        coalition: o.enumeration("coalition", coalition)?,
        country: o.string("country")?,
        country_id: o.integer("country_id")?,
        airbase: o.string("airbase")?,
        start: o.enumeration("start", start_type)?,
        units: o.list("units", unit)?,
        group_id: o.integer("group_id")?,
    };
    o.finish()?;
    Ok(p)
}

fn max_ids(v: &Value, path: &str) -> Result<pb::MaxIds, JsonError> {
    let mut o = Obj::new(v, path)?;
    let m = pb::MaxIds {
        group_id: o.integer("group_id")?,
        unit_id: o.integer("unit_id")?,
    };
    o.finish()?;
    Ok(m)
}

fn tools(v: &Value, path: &str) -> Result<pb::ToolVersions, JsonError> {
    let mut o = Obj::new(v, path)?;
    let generator = o.string("generator")?;
    let pydcs = o.string("pydcs")?;
    let python = o.string("python")?;
    let extra_path = o.sub("extra");
    let extra = match o.get("extra")? {
        None => Default::default(),
        Some(Value::Object(map)) => map
            .iter()
            .map(|(k, v)| match v {
                Value::String(s) => Ok((k.clone(), s.clone())),
                other => Err(err(
                    &format!("{extra_path}.{k}"),
                    format!("expected a string, got {}", kind(other)),
                )),
            })
            .collect::<Result<_, _>>()?,
        Some(other) => {
            return Err(err(
                &extra_path,
                format!("expected an object, got {}", kind(other)),
            ));
        }
    };
    o.finish()?;
    Ok(pb::ToolVersions {
        generator,
        pydcs,
        python,
        extra,
    })
}

fn manifest(v: &Value, path: &str) -> Result<Manifest, JsonError> {
    let mut o = Obj::new(v, path)?;
    let m = Manifest {
        scenario: o.string("scenario")?,
        schema: o.integer("schema")?,
        schema_sha256: o.string("schema_sha256")?,
        build_id: o.string("build_id")?,
        theatre: o.string("theatre")?,
        start_time: o.string("start_time")?,
        time_limit_s: o.integer("time_limit_s")?,
        groups: o.list("groups", group)?,
        zones: o.list("zones", zone)?,
        templates: o.list("templates", template)?,
        flags: o.list("flags", flag)?,
        airbases: o.list("airbases", airbase)?,
        menus: o.list("menus", menu)?,
        player_flights: o.list("player_flights", player_flight)?,
        max_ids: o.message("max_ids", max_ids)?,
        tools: o.message("tools", tools)?,
    };
    o.finish()?;
    Ok(m)
}

// ----- encoding --------------------------------------------------------------------------------

/// A JSON tree in output order.
enum J {
    Obj(Vec<(&'static str, J)>),
    Map(BTreeMap<String, J>),
    Arr(Vec<J>),
    Str(String),
    Raw(String),
}

#[derive(Default)]
struct Fields(Vec<(&'static str, J)>);

impl Fields {
    fn string(mut self, name: &'static str, v: &str) -> Self {
        if !v.is_empty() {
            self.0.push((name, J::Str(v.to_owned())));
        }
        self
    }

    fn int(mut self, name: &'static str, v: impl Into<i64>) -> Self {
        let v = v.into();
        if v != 0 {
            self.0.push((name, J::Raw(v.to_string())));
        }
        self
    }

    fn boolean(mut self, name: &'static str, v: bool) -> Self {
        if v {
            self.0.push((name, J::Raw("true".into())));
        }
        self
    }

    fn double(mut self, name: &'static str, v: f64) -> Self {
        // proto3 omits 0.0 (but keeps -0.0, whose bits differ).
        if v.to_bits() != 0 {
            self.0.push((name, double(v)));
        }
        self
    }

    fn enumeration(
        mut self,
        name: &'static str,
        v: i32,
        to_name: fn(i32) -> Option<&'static str>,
    ) -> Self {
        if v != 0 {
            let j = match to_name(v) {
                Some(n) => J::Str(n.to_owned()),
                None => J::Raw(v.to_string()),
            };
            self.0.push((name, j));
        }
        self
    }

    fn message(mut self, name: &'static str, v: Option<J>) -> Self {
        if let Some(j) = v {
            self.0.push((name, j));
        }
        self
    }

    fn list(mut self, name: &'static str, v: Vec<J>) -> Self {
        if !v.is_empty() {
            self.0.push((name, J::Arr(v)));
        }
        self
    }

    fn strings(self, name: &'static str, v: &[String]) -> Self {
        self.list(name, v.iter().map(|s| J::Str(s.clone())).collect())
    }

    fn done(self) -> J {
        J::Obj(self.0)
    }
}

fn double(v: f64) -> J {
    if v.is_nan() {
        J::Str("NaN".into())
    } else if v.is_infinite() {
        J::Str(if v > 0.0 { "Infinity" } else { "-Infinity" }.into())
    } else {
        J::Raw(python_float_repr(v))
    }
}

/// Python's `repr(float)`: shortest round-trip digits, fixed notation for decimal exponents in
/// `-4 <= e < 16`, else `d.ddde±XX`; integral values in fixed notation end in `.0`.
#[must_use]
pub fn python_float_repr(v: f64) -> String {
    if v == 0.0 {
        return if v.is_sign_negative() { "-0.0" } else { "0.0" }.to_owned();
    }
    // Rust's `{:e}` gives the shortest round-trip digits: "-1.2345e-7".
    let sci = format!("{v:e}");
    let (mantissa, exp) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let (sign, mantissa) = match mantissa.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", mantissa),
    };
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let mut out = String::from(sign);
    if !(-4..16).contains(&exp) {
        out.push_str(&digits[..1]);
        if digits.len() > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        let _ = write!(out, "e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs());
        return out;
    }
    let point = exp + 1; // digits before the decimal point
    if point <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', point.unsigned_abs() as usize));
        out.push_str(&digits);
    } else {
        let point = point as usize;
        if digits.len() <= point {
            out.push_str(&digits);
            out.extend(std::iter::repeat_n('0', point - digits.len()));
            out.push_str(".0");
        } else {
            out.push_str(&digits[..point]);
            out.push('.');
            out.push_str(&digits[point..]);
        }
    }
    out
}

fn category_name(v: i32) -> Option<&'static str> {
    v1::Category::try_from(v).ok().map(|c| c.as_str_name())
}

fn coalition_name(v: i32) -> Option<&'static str> {
    v1::Coalition::try_from(v).ok().map(|c| c.as_str_name())
}

fn start_type_name(v: i32) -> Option<&'static str> {
    v1::StartType::try_from(v).ok().map(|c| c.as_str_name())
}

fn j_map_point(p: &v1::MapPoint) -> J {
    Fields::default()
        .double("north", p.north)
        .double("east", p.east)
        .done()
}

fn j_unit(u: &pb::Unit) -> J {
    Fields::default()
        .string("name", &u.name)
        .string("type_name", &u.type_name)
        .message("position", u.position.as_ref().map(j_map_point))
        .double("alt_m", u.alt_m)
        .double("heading_rad", u.heading_rad)
        .int("unit_id", u.unit_id)
        .message(
            "latlon",
            u.latlon.as_ref().map(|l| {
                Fields::default()
                    .double("lat", l.lat)
                    .double("lon", l.lon)
                    .done()
            }),
        )
        .string("skill", &u.skill)
        .done()
}

fn j_manifest(m: &Manifest) -> J {
    let groups = m
        .groups
        .iter()
        .map(|g| {
            Fields::default()
                .string("ident", &g.ident)
                .string("name", &g.name)
                .enumeration("category", g.category, category_name)
                .enumeration("coalition", g.coalition, coalition_name)
                .string("country", &g.country)
                .int("country_id", g.country_id)
                .strings("roles", &g.roles)
                .list("units", g.units.iter().map(j_unit).collect())
                .int("group_id", g.group_id)
                .boolean("late_activation", g.late_activation)
                .done()
        })
        .collect();
    let zones = m
        .zones
        .iter()
        .map(|z| {
            let f = Fields::default()
                .string("ident", &z.ident)
                .string("name", &z.name);
            match &z.shape {
                Some(pb::zone::Shape::Circle(c)) => f.message(
                    "circle",
                    Some(
                        Fields::default()
                            .message("center", c.center.as_ref().map(j_map_point))
                            .double("radius_m", c.radius_m)
                            .done(),
                    ),
                ),
                Some(pb::zone::Shape::Polygon(p)) => f.message(
                    "polygon",
                    Some(
                        Fields::default()
                            .list("points", p.points.iter().map(j_map_point).collect())
                            .done(),
                    ),
                ),
                None => f,
            }
            .done()
        })
        .collect();
    let templates = m
        .templates
        .iter()
        .map(|t| {
            Fields::default()
                .string("ident", &t.ident)
                .string("name", &t.name)
                .enumeration("category", t.category, category_name)
                .enumeration("coalition", t.coalition, coalition_name)
                .string("country", &t.country)
                .int("country_id", t.country_id)
                .strings("roles", &t.roles)
                .list("units", t.units.iter().map(j_unit).collect())
                .string("task", &t.task)
                .string("dcs_group_json", &t.dcs_group_json)
                .done()
        })
        .collect();
    let flags = m
        .flags
        .iter()
        .map(|f| {
            Fields::default()
                .string("ident", &f.ident)
                .string("name", &f.name)
                .int("initial", f.initial)
                .done()
        })
        .collect();
    let airbases = m
        .airbases
        .iter()
        .map(|a| {
            Fields::default()
                .string("ident", &a.ident)
                .string("name", &a.name)
                .message("position", a.position.as_ref().map(j_map_point))
                .int("airbase_id", a.airbase_id)
                .enumeration("coalition", a.coalition, coalition_name)
                .done()
        })
        .collect();
    let menus = m
        .menus
        .iter()
        .map(|x| {
            Fields::default()
                .string("ident", &x.ident)
                .string("label", &x.label)
                .strings("path", &x.path)
                .done()
        })
        .collect();
    let flights = m
        .player_flights
        .iter()
        .map(|p| {
            Fields::default()
                .string("name", &p.name)
                .string("type_name", &p.type_name)
                .int("size", p.size)
                .enumeration("coalition", p.coalition, coalition_name)
                .string("country", &p.country)
                .int("country_id", p.country_id)
                .string("airbase", &p.airbase)
                .enumeration("start", p.start, start_type_name)
                .list("units", p.units.iter().map(j_unit).collect())
                .int("group_id", p.group_id)
                .done()
        })
        .collect();
    Fields::default()
        .string("scenario", &m.scenario)
        .int("schema", m.schema)
        .string("schema_sha256", &m.schema_sha256)
        .string("build_id", &m.build_id)
        .string("theatre", &m.theatre)
        .string("start_time", &m.start_time)
        .int("time_limit_s", m.time_limit_s)
        .list("groups", groups)
        .list("zones", zones)
        .list("templates", templates)
        .list("flags", flags)
        .list("airbases", airbases)
        .list("menus", menus)
        .list("player_flights", flights)
        .message(
            "max_ids",
            m.max_ids.as_ref().map(|x| {
                Fields::default()
                    .int("group_id", x.group_id)
                    .int("unit_id", x.unit_id)
                    .done()
            }),
        )
        .message(
            "tools",
            m.tools.as_ref().map(|t| {
                let mut f = Fields::default()
                    .string("generator", &t.generator)
                    .string("pydcs", &t.pydcs)
                    .string("python", &t.python);
                if !t.extra.is_empty() {
                    f.0.push((
                        "extra",
                        J::Map(
                            t.extra
                                .iter()
                                .map(|(k, v)| (k.clone(), J::Str(v.clone())))
                                .collect(),
                        ),
                    ));
                }
                f.done()
            }),
        )
        .done()
}

/// Python `json.dumps(..., ensure_ascii=True)` string escaping.
fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (c as u32) >= 0x80 => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    let _ = write!(out, "\\u{unit:04x}");
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write_j(out: &mut String, j: &J, depth: usize) {
    let pad = |out: &mut String, d: usize| {
        out.push('\n');
        out.extend(std::iter::repeat_n(' ', 2 * d));
    };
    match j {
        J::Obj(fields) if fields.is_empty() => out.push_str("{}"),
        J::Map(fields) if fields.is_empty() => out.push_str("{}"),
        J::Obj(fields) => {
            out.push('{');
            for (i, (k, v)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                pad(out, depth + 1);
                write_str(out, k);
                out.push_str(": ");
                write_j(out, v, depth + 1);
            }
            pad(out, depth);
            out.push('}');
        }
        J::Map(fields) => {
            out.push('{');
            for (i, (k, v)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                pad(out, depth + 1);
                write_str(out, k);
                out.push_str(": ");
                write_j(out, v, depth + 1);
            }
            pad(out, depth);
            out.push('}');
        }
        J::Arr(items) if items.is_empty() => out.push_str("[]"),
        J::Arr(items) => {
            out.push('[');
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                pad(out, depth + 1);
                write_j(out, v, depth + 1);
            }
            pad(out, depth);
            out.push(']');
        }
        J::Str(s) => write_str(out, s),
        J::Raw(r) => out.push_str(r),
    }
}

/// `manifest.json` text, byte-identical to the Python generator's output (see module docs).
#[must_use]
pub fn manifest_to_json(m: &Manifest) -> String {
    let mut out = String::new();
    write_j(&mut out, &j_manifest(m), 0);
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_float_repr_matches_python() {
        // Values checked against CPython 3.13 `repr`.
        let cases = [
            (500.0, "500.0"),
            (-32798.01604360901, "-32798.01604360901"),
            (0.1, "0.1"),
            (1e-4, "0.0001"),
            (1e-5, "1e-05"),
            (1.5e-7, "1.5e-07"),
            (1e15, "1000000000000000.0"),
            (1e16, "1e+16"),
            (1.2345678901234568e17, "1.2345678901234568e+17"),
            (123456.789, "123456.789"),
            (-0.0, "-0.0"),
            (0.0, "0.0"),
            (6096.0, "6096.0"),
            (5e-324, "5e-324"),
            (1.7976931348623157e308, "1.7976931348623157e+308"),
            (44.61999999999999, "44.61999999999999"),
        ];
        for (v, want) in cases {
            assert_eq!(python_float_repr(v), want, "{v:e}");
        }
    }

    #[test]
    fn camel_names_and_errors() {
        assert_eq!(camel_case("schema_sha256"), "schemaSha256");
        assert_eq!(camel_case("player_flights"), "playerFlights");
        let m =
            manifest_from_json(r#"{"scenario": "x", "timeLimitS": "60", "maxIds": {"unitId": 3}}"#)
                .unwrap();
        assert_eq!(m.time_limit_s, 60);
        assert_eq!(m.max_ids.unwrap().unit_id, 3);
        let e = manifest_from_json(r#"{"groups": [{"units": [{"alt_m": "high"}]}]}"#).unwrap_err();
        assert_eq!(e.path, "groups[0].units[0].alt_m");
        let e = manifest_from_json(r#"{"zones": [{"circel": {}}]}"#).unwrap_err();
        assert_eq!(e.path, "zones[0].circel");
        assert_eq!(e.message, "unknown field");
        let e = manifest_from_json(r#"{"groups": [{"category": "CATEGORY_TANK"}]}"#).unwrap_err();
        assert!(e.to_string().contains("CATEGORY_TANK"), "{e}");
        let e = manifest_from_json(r#"{"schema": -1}"#).unwrap_err();
        assert!(e.message.contains("out of range"), "{e}");
        assert!(manifest_from_json("[]").is_err());
    }

    #[test]
    fn strings_are_escaped_like_python() {
        let mut s = String::new();
        write_str(&mut s, "Майкоп \"A\"\n\u{1f6e9}\u{7f}\u{1}");
        assert_eq!(
            s,
            "\"\\u041c\\u0430\\u0439\\u043a\\u043e\\u043f \\\"A\\\"\\n\\ud83d\\udee9\u{7f}\\u0001\""
        );
    }
}
