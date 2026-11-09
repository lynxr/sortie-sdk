//! Structural validation of a manifest: what the engine checks before it accepts a build and
//! what the fixtures are tested against.
//!
//! Rules:
//!
//! * **Names.** Scenario `[a-z][a-z0-9_]*`; symbols `[A-Z][A-Z0-9_]*`, unique per kind; roles
//!   `[a-z][a-z0-9_-]*`, sorted and unique. DCS group names are unique among groups and player
//!   flights, unit names among all units (pydcs does not check, and `coalition.addGroup` with a
//!   taken name destroys the other object). Template names, flag names, zone names, airbase
//!   names are unique within their kind; template names never contain `#` (reserved for
//!   `<template>#<session4>-<n>`).
//! * **Ids.** Placed groups and units carry the ids written into the .miz: `group_id > 0`
//!   unique across groups and player flights, `unit_id > 0` unique across all placed units;
//!   `max_ids` covers them (ids are never renumbered). Template units have `unit_id == 0`
//!   (runtime spawns never set ids; DCS assigns them).
//! * **Geometry.** Positions and altitudes finite and inside the theatre plane (±[`MAX_COORD_M`]),
//!   lat/lon in range; circles with a finite radius > 0; polygons with ≥ 3 finite vertices,
//!   non-zero area and no self-intersection.
//! * **Templates are spawnable.** Kind category, coalition and country set, at least one unit
//!   with a type, and `dcs_group_json` (when present) a JSON object whose `units` (if any) match
//!   the template's unit count.
//! * **Header.** Schema ≥ 1, `schema_sha256` = hash of the symbols, `build_id` a ULID,
//!   `start_time` RFC 3339 UTC (`2026-11-01T06:30:00Z`), theatre set.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use sortie_proto::manifest::v1 as pb;
use sortie_proto::v1;
use sortie_types::{MapPoint, ZoneShape};

use crate::lock::{ScenarioLock, check_ident, check_role, check_scenario_name, kind_of_category};
use crate::naming;

/// Largest |north| or |east| accepted for a map position, metres (every DCS theatre fits).
pub const MAX_COORD_M: f64 = 5_000_000.0;

/// One validation finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    /// Dotted path, e.g. `groups[0].units[3].name`.
    pub path: String,
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// A manifest that failed [`validate`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid manifest ({} problem{}):\n{}", .0.len(), if .0.len() == 1 { "" } else { "s" },
        .0.iter().map(|i| format!("  - {i}")).collect::<Vec<_>>().join("\n"))]
pub struct InvalidManifest(pub Vec<Issue>);

/// [`validate`] as a `Result`.
pub fn ensure_valid(m: &pb::Manifest) -> Result<(), InvalidManifest> {
    let issues = validate(m);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(InvalidManifest(issues))
    }
}

struct Checker {
    issues: Vec<Issue>,
}

impl Checker {
    fn push(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.issues.push(Issue {
            path: path.into(),
            message: message.into(),
        });
    }

    /// Records `name` in `seen` (first path wins) and reports duplicates.
    fn unique(
        &mut self,
        seen: &mut BTreeMap<String, String>,
        path: String,
        name: &str,
        what: &str,
        why: &str,
    ) {
        if name.is_empty() {
            return;
        }
        match seen.get(name) {
            Some(first) => {
                let msg = format!("duplicate {what} {name:?} (also at {first}){why}");
                self.push(path, msg);
            }
            None => {
                seen.insert(name.to_owned(), path);
            }
        }
    }

    fn dcs_name(&mut self, path: &str, name: &str) {
        if let Err(e) = naming::check_dcs_name(name) {
            self.push(path, e.to_string().replace("group name", "name"));
        }
    }

    fn ident(
        &mut self,
        path: String,
        what: &'static str,
        ident: &str,
        seen: &mut BTreeSet<String>,
    ) {
        if let Err(e) = check_ident(what, ident) {
            self.push(path, e.to_string());
        } else if !seen.insert(ident.to_owned()) {
            self.push(path, format!("duplicate {what} symbol {ident:?}"));
        }
    }

    fn roles(&mut self, path: String, roles: &[String]) {
        for r in roles {
            if let Err(e) = check_role(r) {
                self.push(path.clone(), e.to_string());
            }
        }
        if roles.windows(2).any(|w| w[0] >= w[1]) {
            self.push(path, "roles must be sorted and unique");
        }
    }

    fn point(&mut self, path: String, p: Option<&v1::MapPoint>, required: bool) {
        match p {
            None if required => self.push(path, "position is missing"),
            None => {}
            Some(p) => {
                let mp = MapPoint::from(*p);
                if !mp.is_finite() {
                    self.push(path, "coordinates must be finite");
                } else if mp.north.abs() > MAX_COORD_M || mp.east.abs() > MAX_COORD_M {
                    self.push(
                        path,
                        format!(
                            "({}, {}) is outside the theatre plane (|coordinate| <= {MAX_COORD_M} m)",
                            mp.north, mp.east
                        ),
                    );
                }
            }
        }
    }

    fn unit(&mut self, path: &str, u: &pb::Unit) {
        self.dcs_name(&format!("{path}.name"), &u.name);
        if u.type_name.is_empty() {
            self.push(format!("{path}.type_name"), "must not be empty");
        }
        self.point(format!("{path}.position"), u.position.as_ref(), true);
        if !u.alt_m.is_finite() {
            self.push(format!("{path}.alt_m"), "must be finite");
        }
        if !u.heading_rad.is_finite() {
            self.push(format!("{path}.heading_rad"), "must be finite");
        }
        if let Some(ll) = &u.latlon
            && !(ll.lat.is_finite()
                && ll.lon.is_finite()
                && (-90.0..=90.0).contains(&ll.lat)
                && (-180.0..=180.0).contains(&ll.lon))
        {
            self.push(format!("{path}.latlon"), "latitude/longitude out of range");
        }
    }

    fn category(&mut self, path: String, category: i32) {
        if kind_of_category(category).is_none() {
            self.push(path, "must be AIRPLANE, HELICOPTER, GROUND_UNIT or SHIP");
        }
    }

    fn coalition(&mut self, path: String, coalition: i32) {
        if sortie_types::Coalition::from_i32(coalition).is_none() {
            self.push(path, "coalition must be RED, BLUE or NEUTRAL");
        }
    }
}

/// Checks a .miz id: > 0, unique in `ids`, and tracks the maximum.
fn placed_id(
    c: &mut Checker,
    ids: &mut BTreeMap<u32, String>,
    max: &mut u32,
    path: String,
    id: u32,
    what: &str,
) {
    if id == 0 {
        c.push(
            path,
            format!("placed {what}s need the {what}Id written into the .miz (> 0)"),
        );
        return;
    }
    *max = (*max).max(id);
    if let Some(first) = ids.get(&id) {
        let msg = format!("duplicate {what}Id {id} (also at {first})");
        c.push(path, msg);
    } else {
        ids.insert(id, path);
    }
}

const ADD_GROUP: &str = " (coalition.addGroup would destroy the other object)";

/// Returns every problem found (empty = valid); see the module docs for the rules.
#[must_use]
pub fn validate(m: &pb::Manifest) -> Vec<Issue> {
    let mut c = Checker { issues: Vec::new() };
    header(&mut c, m);

    let mut group_names = BTreeMap::new();
    let mut unit_names = BTreeMap::new();
    let mut group_ids: BTreeMap<u32, String> = BTreeMap::new();
    let mut unit_ids: BTreeMap<u32, String> = BTreeMap::new();
    let mut max_group_id = 0;
    let mut max_unit_id = 0;
    let mut group_idents = BTreeSet::new();
    for (i, g) in m.groups.iter().enumerate() {
        let p = format!("groups[{i}]");
        c.dcs_name(&format!("{p}.name"), &g.name);
        c.unique(
            &mut group_names,
            format!("{p}.name"),
            &g.name,
            "group name",
            ADD_GROUP,
        );
        if !g.ident.is_empty() {
            c.ident(format!("{p}.ident"), "group", &g.ident, &mut group_idents);
        }
        c.category(format!("{p}.category"), g.category);
        c.coalition(format!("{p}.coalition"), g.coalition);
        c.roles(format!("{p}.roles"), &g.roles);
        placed_id(
            &mut c,
            &mut group_ids,
            &mut max_group_id,
            format!("{p}.group_id"),
            g.group_id,
            "group",
        );
        if g.units.is_empty() {
            c.push(format!("{p}.units"), "a group needs at least one unit");
        }
        for (j, u) in g.units.iter().enumerate() {
            let up = format!("{p}.units[{j}]");
            c.unit(&up, u);
            c.unique(
                &mut unit_names,
                format!("{up}.name"),
                &u.name,
                "unit name",
                ADD_GROUP,
            );
            placed_id(
                &mut c,
                &mut unit_ids,
                &mut max_unit_id,
                format!("{up}.unit_id"),
                u.unit_id,
                "unit",
            );
        }
    }
    for (i, f) in m.player_flights.iter().enumerate() {
        let p = format!("player_flights[{i}]");
        c.dcs_name(&format!("{p}.name"), &f.name);
        c.unique(
            &mut group_names,
            format!("{p}.name"),
            &f.name,
            "group name",
            ADD_GROUP,
        );
        if f.type_name.is_empty() {
            c.push(format!("{p}.type_name"), "must not be empty");
        }
        c.coalition(format!("{p}.coalition"), f.coalition);
        if f.airbase.is_empty() {
            c.push(format!("{p}.airbase"), "must not be empty");
        }
        if sortie_types::StartType::from_i32(f.start).is_none() {
            c.push(
                format!("{p}.start"),
                "start type must be RUNWAY, HOT, COLD or AIR",
            );
        }
        if f.size == 0 {
            c.push(format!("{p}.size"), "a flight needs at least one aircraft");
        } else if !f.units.is_empty() && f.units.len() != f.size as usize {
            c.push(
                format!("{p}.units"),
                format!("has {} units but size is {}", f.units.len(), f.size),
            );
        }
        placed_id(
            &mut c,
            &mut group_ids,
            &mut max_group_id,
            format!("{p}.group_id"),
            f.group_id,
            "group",
        );
        for (j, u) in f.units.iter().enumerate() {
            let up = format!("{p}.units[{j}]");
            c.unit(&up, u);
            c.unique(
                &mut unit_names,
                format!("{up}.name"),
                &u.name,
                "unit name",
                ADD_GROUP,
            );
            placed_id(
                &mut c,
                &mut unit_ids,
                &mut max_unit_id,
                format!("{up}.unit_id"),
                u.unit_id,
                "unit",
            );
        }
    }
    match &m.max_ids {
        None if max_group_id > 0 || max_unit_id > 0 => {
            c.push(
                "max_ids",
                "missing (the generator records the highest ids it wrote)",
            );
        }
        None => {}
        Some(ids) => {
            if ids.group_id < max_group_id {
                c.push(
                    "max_ids.group_id",
                    format!("is {} but groupId {max_group_id} is used", ids.group_id),
                );
            }
            if ids.unit_id < max_unit_id {
                c.push(
                    "max_ids.unit_id",
                    format!("is {} but unitId {max_unit_id} is used", ids.unit_id),
                );
            }
        }
    }

    zones(&mut c, m);
    templates(&mut c, m);
    simple_symbols(&mut c, m);

    match ScenarioLock::from_manifest(m, "") {
        Ok(lock) if lock.schema_sha256 != m.schema_sha256 => c.push(
            "schema_sha256",
            format!(
                "is {:?} but the symbols hash to {:?}",
                m.schema_sha256, lock.schema_sha256
            ),
        ),
        Ok(_) => {}
        // Name and kind problems are already reported field by field above.
        Err(_) if !c.issues.is_empty() => {}
        Err(e) => c.push("symbols", e.to_string()),
    }
    c.issues
}

fn header(c: &mut Checker, m: &pb::Manifest) {
    if let Err(e) = check_scenario_name(&m.scenario) {
        c.push("scenario", e.to_string());
    }
    if m.schema == 0 {
        c.push("schema", "must be >= 1");
    }
    if !is_ulid(&m.build_id) {
        c.push(
            "build_id",
            format!(
                "{:?} is not a ULID (26 Crockford base32 characters)",
                m.build_id
            ),
        );
    }
    if m.theatre.is_empty() {
        c.push("theatre", "must not be empty");
    }
    if !is_rfc3339_utc(&m.start_time) {
        c.push(
            "start_time",
            format!(
                "{:?} is not RFC 3339 UTC, e.g. 2026-11-01T06:30:00Z",
                m.start_time
            ),
        );
    }
}

fn zones(c: &mut Checker, m: &pb::Manifest) {
    let mut idents = BTreeSet::new();
    let mut names = BTreeMap::new();
    for (i, z) in m.zones.iter().enumerate() {
        let p = format!("zones[{i}]");
        c.ident(format!("{p}.ident"), "zone", &z.ident, &mut idents);
        if z.name.is_empty() {
            c.push(
                format!("{p}.name"),
                "must not be empty (the trigger zone name in the .miz)",
            );
        }
        c.unique(&mut names, format!("{p}.name"), &z.name, "zone name", "");
        match &z.shape {
            None => c.push(format!("{p}.shape"), "zone needs a circle or a polygon"),
            Some(pb::zone::Shape::Circle(circle)) => {
                c.point(format!("{p}.circle.center"), circle.center.as_ref(), true);
                if !(circle.radius_m.is_finite() && circle.radius_m > 0.0) {
                    c.push(format!("{p}.circle.radius_m"), "must be finite and > 0");
                }
            }
            Some(pb::zone::Shape::Polygon(poly)) => {
                let points: Vec<MapPoint> = poly.points.iter().copied().map(Into::into).collect();
                if points.len() < 3 {
                    c.push(
                        format!("{p}.polygon.points"),
                        "a polygon needs at least 3 points",
                    );
                } else if points.iter().any(|q| !q.is_finite()) {
                    c.push(format!("{p}.polygon.points"), "coordinates must be finite");
                } else if let Some((a, b)) = self_intersection(&points) {
                    c.push(
                        format!("{p}.polygon.points"),
                        format!("edges {a} and {b} cross (the polygon intersects itself)"),
                    );
                } else if !(ZoneShape::Polygon { points: &points }).is_valid() {
                    c.push(format!("{p}.polygon.points"), "the polygon has zero area");
                } else {
                    for (j, q) in poly.points.iter().enumerate() {
                        c.point(format!("{p}.polygon.points[{j}]"), Some(q), true);
                    }
                }
            }
        }
    }
}

fn templates(c: &mut Checker, m: &pb::Manifest) {
    let mut idents = BTreeSet::new();
    let mut names = BTreeMap::new();
    for (i, t) in m.templates.iter().enumerate() {
        let p = format!("templates[{i}]");
        c.ident(format!("{p}.ident"), "template", &t.ident, &mut idents);
        if t.name.is_empty() || t.name.contains(naming::RUNTIME_SEPARATOR) {
            c.push(
                format!("{p}.name"),
                "must be non-empty and must not contain '#' (spawned groups are named <template>#<session4>-<n>)",
            );
        } else {
            c.dcs_name(&format!("{p}.name"), &t.name);
        }
        c.unique(
            &mut names,
            format!("{p}.name"),
            &t.name,
            "template name",
            "",
        );
        c.category(format!("{p}.category"), t.category);
        c.coalition(format!("{p}.coalition"), t.coalition);
        if t.country.is_empty() {
            c.push(
                format!("{p}.country"),
                "must not be empty (spawn_group needs a country)",
            );
        }
        c.roles(format!("{p}.roles"), &t.roles);
        if t.units.is_empty() {
            c.push(format!("{p}.units"), "a template needs at least one unit");
        }
        for (j, u) in t.units.iter().enumerate() {
            let up = format!("{p}.units[{j}]");
            if u.type_name.is_empty() {
                c.push(format!("{up}.type_name"), "must not be empty");
            }
            c.point(format!("{up}.position"), u.position.as_ref(), false);
            if !u.alt_m.is_finite() || !u.heading_rad.is_finite() {
                c.push(up.clone(), "altitude and heading must be finite");
            }
            if u.unit_id != 0 {
                c.push(
                    format!("{up}.unit_id"),
                    "template units must not carry a unitId (DCS assigns ids to runtime spawns)",
                );
            }
        }
        if !t.dcs_group_json.is_empty() {
            match serde_json::from_str::<serde_json::Value>(&t.dcs_group_json) {
                Ok(serde_json::Value::Object(obj)) => {
                    if let Some(units) = obj.get("units") {
                        let n = units.as_array().map(Vec::len).or_else(|| {
                            // pydcs dicts become Lua-style {"1": {...}, "2": {...}} tables.
                            units.as_object().map(serde_json::Map::len)
                        });
                        if n.is_some_and(|n| n != t.units.len()) {
                            c.push(
                                format!("{p}.dcs_group_json"),
                                format!(
                                    "has {} units but the template lists {}",
                                    n.unwrap_or(0),
                                    t.units.len()
                                ),
                            );
                        }
                    }
                }
                Ok(_) => c.push(format!("{p}.dcs_group_json"), "must be a JSON object"),
                Err(e) => c.push(
                    format!("{p}.dcs_group_json"),
                    format!("is not valid JSON: {e}"),
                ),
            }
        }
    }
}

fn simple_symbols(c: &mut Checker, m: &pb::Manifest) {
    let mut idents = BTreeSet::new();
    let mut names = BTreeMap::new();
    for (i, f) in m.flags.iter().enumerate() {
        let p = format!("flags[{i}]");
        c.ident(format!("{p}.ident"), "flag", &f.ident, &mut idents);
        if f.name.is_empty() {
            c.push(format!("{p}.name"), "must not be empty (the DCS flag name)");
        }
        c.unique(&mut names, format!("{p}.name"), &f.name, "flag name", "");
    }
    let mut idents = BTreeSet::new();
    let mut names = BTreeMap::new();
    for (i, a) in m.airbases.iter().enumerate() {
        let p = format!("airbases[{i}]");
        c.ident(format!("{p}.ident"), "airbase", &a.ident, &mut idents);
        if a.name.is_empty() {
            c.push(
                format!("{p}.name"),
                "must not be empty (the DCS airbase name)",
            );
        }
        c.unique(&mut names, format!("{p}.name"), &a.name, "airbase name", "");
        c.point(format!("{p}.position"), a.position.as_ref(), false);
    }
    let mut idents = BTreeSet::new();
    for (i, x) in m.menus.iter().enumerate() {
        let p = format!("menus[{i}]");
        c.ident(format!("{p}.ident"), "menu", &x.ident, &mut idents);
        if x.label.trim().is_empty() {
            c.push(format!("{p}.label"), "must not be empty");
        }
        if x.path.iter().any(|s| s.trim().is_empty()) {
            c.push(format!("{p}.path"), "sub-menu names must not be empty");
        }
    }
}

/// First pair of non-adjacent edges that cross, by edge index (`i` = points[i]→points[i+1]).
fn self_intersection(points: &[MapPoint]) -> Option<(usize, usize)> {
    let n = points.len();
    let edge = |i: usize| (points[i], points[(i + 1) % n]);
    let orient = |a: MapPoint, b: MapPoint, c: MapPoint| {
        let v = (b.north - a.north) * (c.east - a.east) - (b.east - a.east) * (c.north - a.north);
        if v > 0.0 {
            1
        } else if v < 0.0 {
            -1
        } else {
            0
        }
    };
    let on = |a: MapPoint, b: MapPoint, p: MapPoint| {
        p.north >= a.north.min(b.north)
            && p.north <= a.north.max(b.north)
            && p.east >= a.east.min(b.east)
            && p.east <= a.east.max(b.east)
    };
    for i in 0..n {
        for j in (i + 1)..n {
            // Adjacent edges share a vertex by construction.
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let ((a, b), (c2, d)) = (edge(i), edge(j));
            let (o1, o2, o3, o4) = (
                orient(a, b, c2),
                orient(a, b, d),
                orient(c2, d, a),
                orient(c2, d, b),
            );
            let crosses = (o1 != o2 && o3 != o4)
                || (o1 == 0 && on(a, b, c2))
                || (o2 == 0 && on(a, b, d))
                || (o3 == 0 && on(c2, d, a))
                || (o4 == 0 && on(c2, d, b));
            if crosses {
                return Some((i, j));
            }
        }
    }
    None
}

/// 26 characters of Crockford base32 (no I, L, O, U), first character ≤ 7.
#[must_use]
pub fn is_ulid(s: &str) -> bool {
    s.len() == 26
        && s.bytes().enumerate().all(|(i, b)| {
            let crockford = b.is_ascii_digit()
                || (b.is_ascii_uppercase() && !matches!(b, b'I' | b'L' | b'O' | b'U'));
            crockford && (i > 0 || b <= b'7')
        })
}

/// `YYYY-MM-DDTHH:MM:SS[.fff]Z` with plausible field ranges.
#[must_use]
pub fn is_rfc3339_utc(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 20 || b[b.len() - 1] != b'Z' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| -> Option<u32> {
        let part = s.get(r)?;
        part.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| part.parse().ok())
            .flatten()
    };
    let seps = b[4] == b'-' && b[7] == b'-' && b[10] == b'T' && b[13] == b':' && b[16] == b':';
    let frac = &s[19..s.len() - 1];
    let frac_ok = frac.is_empty()
        || (frac.len() > 1
            && frac.starts_with('.')
            && frac[1..].bytes().all(|c| c.is_ascii_digit()));
    seps && frac_ok
        && num(0..4).is_some()
        && num(5..7).is_some_and(|m| (1..=12).contains(&m))
        && num(8..10).is_some_and(|d| (1..=31).contains(&d))
        && num(11..13).is_some_and(|h| h <= 23)
        && num(14..16).is_some_and(|m| m <= 59)
        && num(17..19).is_some_and(|s| s <= 60)
}
