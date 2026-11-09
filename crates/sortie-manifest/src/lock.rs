//! `scenario.lock.json`: the symbol set of a scenario, its kinds and roles, the canonical schema
//! hash, and (outside the hash) zone geometry for code generation.
//!
//! # Canonical schema hash (must match `sortie-gen` byte for byte)
//!
//! ```text
//! schema_sha256 = lowercase_hex(sha256(utf8(canonical)))
//! canonical     = json.dumps({"scenario": <str>, "schema": <int>, "symbols": <symbols>},
//!                            sort_keys=True, ensure_ascii=False, separators=(",", ":"))
//! <symbols>     = {"airbases": [{"ident"}], "flags": [{"ident"}],
//!                  "groups": [{"ident", "kind", "roles"}], "menus": [{"ident"}],
//!                  "templates": [{"ident", "kind", "roles"}], "zones": [{"ident"}]}
//! ```
//!
//! All six arrays are always present, each sorted by `ident`; `roles` are sorted and unique.
//! The hashed part contains only strings and integers (no floats). Geometry is excluded.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sortie_proto::manifest::v1 as pb;
use sortie_proto::v1;
use sortie_types::MapPoint;

/// Current `format` of scenario.lock.json.
pub const LOCK_FORMAT: u32 = 1;

/// Typed `scenario.lock.json`. Field order is alphabetical so that `serde_json` output has the
/// same key order as Python's `json.dumps(..., sort_keys=True)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioLock {
    pub format: u32,
    /// Free-form, e.g. "sortie-gen 0.1.0". Not hashed.
    #[serde(default)]
    pub generator: String,
    /// Not hashed; used by `sortie-build` for const zone geometry.
    #[serde(default)]
    pub geometry: GeometryLock,
    pub scenario: String,
    pub schema: u32,
    pub schema_sha256: String,
    pub symbols: SymbolSet,
}

/// The hashed symbol set.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SymbolSet {
    pub airbases: Vec<IdentSymbol>,
    pub flags: Vec<IdentSymbol>,
    pub groups: Vec<KindSymbol>,
    pub menus: Vec<IdentSymbol>,
    pub templates: Vec<KindSymbol>,
    pub zones: Vec<IdentSymbol>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentSymbol {
    pub ident: String,
}

/// A group or template symbol: kind is "ground" | "air" | "helo" | "ship".
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindSymbol {
    pub ident: String,
    pub kind: String,
    #[serde(default)]
    pub roles: Vec<String>,
}

/// Geometry outside the hash.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryLock {
    #[serde(default)]
    pub zones: BTreeMap<String, ZoneGeometry>,
}

/// `{"circle": {"center": {"east", "north"}, "radius_m"}}` or `{"polygon": {"points": [...]}}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ZoneGeometry {
    Circle { center: MapPoint, radius_m: f64 },
    Polygon { points: Vec<MapPoint> },
}

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid scenario.lock.json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported scenario.lock.json format {0} (this build understands {LOCK_FORMAT})")]
    Format(u32),
    #[error("invalid {what} {value:?}: {rule}")]
    BadName {
        what: &'static str,
        value: String,
        rule: &'static str,
    },
    #[error("duplicate {what} symbol {ident:?}")]
    Duplicate { what: &'static str, ident: String },
    #[error("unknown kind {kind:?} for {ident:?} (expected ground, air, helo or ship)")]
    BadKind { ident: String, kind: String },
    #[error("schema_sha256 mismatch: file says {stored}, symbols hash to {computed}")]
    HashMismatch { stored: String, computed: String },
    #[error("group/template {ident:?} has category {category} which has no group kind")]
    BadCategory { ident: String, category: i32 },
}

impl SymbolSet {
    /// The symbols of a manifest (kinds from categories; groups without ident are skipped,
    /// unknown categories get the kind `"?"`), canonicalized.
    #[must_use]
    pub fn of_manifest(m: &pb::Manifest) -> Self {
        let ident = |s: &str| IdentSymbol {
            ident: s.to_owned(),
        };
        let kind = |ident: &str, category: i32, roles: &[String]| KindSymbol {
            ident: ident.to_owned(),
            kind: kind_of_category(category).unwrap_or("?").to_owned(),
            roles: roles.to_vec(),
        };
        let mut s = SymbolSet {
            airbases: m.airbases.iter().map(|a| ident(&a.ident)).collect(),
            flags: m.flags.iter().map(|f| ident(&f.ident)).collect(),
            groups: m
                .groups
                .iter()
                .filter(|g| !g.ident.is_empty())
                .map(|g| kind(&g.ident, g.category, &g.roles))
                .collect(),
            menus: m.menus.iter().map(|x| ident(&x.ident)).collect(),
            templates: m
                .templates
                .iter()
                .map(|t| kind(&t.ident, t.category, &t.roles))
                .collect(),
            zones: m.zones.iter().map(|z| ident(&z.ident)).collect(),
        };
        s.canonicalize();
        s
    }

    /// `"<kind>:<IDENT>" -> (group kind or "", roles)` for every symbol. The `<kind>` prefix
    /// matches `sortie_types::SymbolKind::as_str`.
    #[must_use]
    pub fn entries(&self) -> BTreeMap<String, (String, Vec<String>)> {
        let mut out = BTreeMap::new();
        for (what, list) in [
            ("airbase", &self.airbases),
            ("flag", &self.flags),
            ("menu", &self.menus),
            ("zone", &self.zones),
        ] {
            for s in list {
                out.insert(format!("{what}:{}", s.ident), (String::new(), Vec::new()));
            }
        }
        for (what, list) in [("group", &self.groups), ("template", &self.templates)] {
            for s in list {
                out.insert(
                    format!("{what}:{}", s.ident),
                    (s.kind.clone(), s.roles.clone()),
                );
            }
        }
        out
    }

    /// Sorts every array by ident and every role list, removing duplicate roles.
    pub fn canonicalize(&mut self) {
        for v in [
            &mut self.airbases,
            &mut self.flags,
            &mut self.menus,
            &mut self.zones,
        ] {
            v.sort();
        }
        for v in [&mut self.groups, &mut self.templates] {
            for s in v.iter_mut() {
                let roles: BTreeSet<String> = s.roles.drain(..).collect();
                s.roles = roles.into_iter().collect();
            }
            v.sort();
        }
    }

    /// Checks names, kinds and uniqueness.
    pub fn check(&self) -> Result<(), LockError> {
        let ident_lists: [(&'static str, Vec<&str>); 6] = [
            (
                "airbase",
                self.airbases.iter().map(|s| s.ident.as_str()).collect(),
            ),
            (
                "flag",
                self.flags.iter().map(|s| s.ident.as_str()).collect(),
            ),
            (
                "group",
                self.groups.iter().map(|s| s.ident.as_str()).collect(),
            ),
            (
                "menu",
                self.menus.iter().map(|s| s.ident.as_str()).collect(),
            ),
            (
                "template",
                self.templates.iter().map(|s| s.ident.as_str()).collect(),
            ),
            (
                "zone",
                self.zones.iter().map(|s| s.ident.as_str()).collect(),
            ),
        ];
        for (what, idents) in ident_lists {
            let mut seen = BTreeSet::new();
            for ident in idents {
                check_ident(what, ident)?;
                if !seen.insert(ident) {
                    return Err(LockError::Duplicate {
                        what,
                        ident: ident.to_owned(),
                    });
                }
            }
        }
        for s in self.groups.iter().chain(&self.templates) {
            if sortie_types::kind::category_of(&s.kind).is_none() {
                return Err(LockError::BadKind {
                    ident: s.ident.clone(),
                    kind: s.kind.clone(),
                });
            }
            for r in &s.roles {
                check_role(r)?;
            }
        }
        Ok(())
    }
}

/// `[A-Z][A-Z0-9_]*`
pub fn check_ident(what: &'static str, ident: &str) -> Result<(), LockError> {
    let mut chars = ident.chars();
    let ok = matches!(chars.next(), Some('A'..='Z'))
        && chars.all(|c| matches!(c, 'A'..='Z' | '0'..='9' | '_'));
    if ok {
        Ok(())
    } else {
        Err(LockError::BadName {
            what,
            value: ident.to_owned(),
            rule: "symbols must match [A-Z][A-Z0-9_]*",
        })
    }
}

/// `[a-z][a-z0-9_]*`
pub fn check_scenario_name(name: &str) -> Result<(), LockError> {
    let mut chars = name.chars();
    let ok = matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'));
    if ok {
        Ok(())
    } else {
        Err(LockError::BadName {
            what: "scenario name",
            value: name.to_owned(),
            rule: "must match [a-z][a-z0-9_]*",
        })
    }
}

/// `[a-z][a-z0-9_-]*`
pub fn check_role(role: &str) -> Result<(), LockError> {
    let mut chars = role.chars();
    let ok = matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '-'));
    if ok {
        Ok(())
    } else {
        Err(LockError::BadName {
            what: "role",
            value: role.to_owned(),
            rule: "roles must match [a-z][a-z0-9_-]*",
        })
    }
}

/// Lock-file kind of a manifest category.
#[must_use]
pub fn kind_of_category(category: i32) -> Option<&'static str> {
    use sortie_types::{Air, Ground, Helo, Kind, Ship};
    match v1::Category::try_from(category).ok()? {
        v1::Category::Airplane => Some(Air::NAME),
        v1::Category::Helicopter => Some(Helo::NAME),
        v1::Category::GroundUnit => Some(Ground::NAME),
        v1::Category::Ship => Some(Ship::NAME),
        _ => None,
    }
}

/// Rebuilds every JSON object with keys inserted in sorted order, so serialization is sorted
/// whether or not serde_json's `preserve_order` feature is enabled in the build.
fn sorted(value: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let entries: BTreeMap<String, Value> =
                map.into_iter().map(|(k, v)| (k, sorted(v))).collect();
            Value::Object(entries.into_iter().collect())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
        other => other,
    }
}

/// The exact bytes that are hashed (see the module docs). `symbols` is canonicalized first.
pub fn canonical_json(
    scenario: &str,
    schema: u32,
    symbols: &SymbolSet,
) -> Result<Vec<u8>, LockError> {
    let mut symbols = symbols.clone();
    symbols.canonicalize();
    let value = serde_json::json!({
        "scenario": scenario,
        "schema": schema,
        "symbols": serde_json::to_value(&symbols)?,
    });
    Ok(serde_json::to_vec(&sorted(value))?)
}

/// Lowercase hex sha256 of [`canonical_json`].
pub fn schema_sha256(
    scenario: &str,
    schema: u32,
    symbols: &SymbolSet,
) -> Result<String, LockError> {
    Ok(hex::encode(Sha256::digest(canonical_json(
        scenario, schema, symbols,
    )?)))
}

/// How a lock file and a manifest disagree (see [`compare`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LockDiff {
    /// `(lock, manifest)` scenario names, when they differ.
    pub scenario: Option<(String, String)>,
    /// `(lock, manifest)` schema numbers, when they differ.
    pub schema: Option<(u32, u32)>,
    /// `(lock, manifest)` hashes, when they differ.
    pub hash: Option<(String, String)>,
    /// Symbols the lock (and so the compiled logic) has but the manifest lacks:
    /// `"<kind>:<IDENT>"`, sorted.
    pub missing: Vec<String>,
    /// Symbols the manifest has but the lock does not (harmless for running logic).
    pub extra: Vec<String>,
    /// `(symbol, lock kind, manifest kind)` for groups/templates whose kind changed.
    pub kind_changed: Vec<(String, String, String)>,
    /// Groups/templates whose roles differ.
    pub roles_changed: Vec<String>,
}

impl LockDiff {
    /// Nothing differs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == LockDiff::default()
    }

    /// Logic built from the lock can run against the manifest: same scenario, no missing
    /// symbol, no kind change. (A hash or schema mismatch alone is only a warning.)
    #[must_use]
    pub fn is_compatible(&self) -> bool {
        self.scenario.is_none() && self.missing.is_empty() && self.kind_changed.is_empty()
    }
}

impl std::fmt::Display for LockDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            return f.write_str("the lock and the manifest agree");
        }
        let mut parts = Vec::new();
        if let Some((l, m)) = &self.scenario {
            parts.push(format!("scenario {l:?} in the lock, {m:?} in the manifest"));
        }
        if let Some((l, m)) = &self.schema {
            parts.push(format!("schema {l} in the lock, {m} in the manifest"));
        }
        if let Some((l, m)) = &self.hash {
            parts.push(format!(
                "schema_sha256 {l} in the lock, {m} in the manifest"
            ));
        }
        if !self.missing.is_empty() {
            parts.push(format!(
                "missing from the manifest: {}",
                self.missing.join(", ")
            ));
        }
        if !self.extra.is_empty() {
            parts.push(format!("not in the lock: {}", self.extra.join(", ")));
        }
        for (s, l, m) in &self.kind_changed {
            parts.push(format!("{s} is {l} in the lock but {m} in the manifest"));
        }
        if !self.roles_changed.is_empty() {
            parts.push(format!("roles differ: {}", self.roles_changed.join(", ")));
        }
        f.write_str(&parts.join("; "))
    }
}

/// Compares the symbols a lock promises with what a manifest provides.
#[must_use]
pub fn compare(lock: &ScenarioLock, m: &pb::Manifest) -> LockDiff {
    let mut d = LockDiff::default();
    if lock.scenario != m.scenario {
        d.scenario = Some((lock.scenario.clone(), m.scenario.clone()));
    }
    if lock.schema != m.schema {
        d.schema = Some((lock.schema, m.schema));
    }
    if lock.schema_sha256 != m.schema_sha256 {
        d.hash = Some((lock.schema_sha256.clone(), m.schema_sha256.clone()));
    }
    let ours = lock.symbols.entries();
    let theirs = SymbolSet::of_manifest(m).entries();
    for (key, (kind, roles)) in &ours {
        match theirs.get(key) {
            None => d.missing.push(key.clone()),
            Some((k2, r2)) => {
                if k2 != kind {
                    d.kind_changed.push((key.clone(), kind.clone(), k2.clone()));
                } else if r2 != roles {
                    d.roles_changed.push(key.clone());
                }
            }
        }
    }
    d.extra = theirs
        .keys()
        .filter(|k| !ours.contains_key(*k))
        .cloned()
        .collect();
    d
}

impl ScenarioLock {
    /// `"<kind>:<IDENT>"` for every symbol, as in `sortie.v1.Attach.symbols_used`.
    #[must_use]
    pub fn symbol_keys(&self) -> Vec<String> {
        self.symbols.entries().into_keys().collect()
    }

    /// Parses and fully checks a lock file (format, names, kinds, hash).
    pub fn from_json_str(s: &str) -> Result<Self, LockError> {
        let lock: ScenarioLock = serde_json::from_str(s)?;
        lock.verify()?;
        Ok(lock)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, LockError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| LockError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_json_str(&text)
    }

    /// Checks format, names, kinds and that `schema_sha256` matches the symbols.
    pub fn verify(&self) -> Result<(), LockError> {
        if self.format != LOCK_FORMAT {
            return Err(LockError::Format(self.format));
        }
        check_scenario_name(&self.scenario)?;
        self.symbols.check()?;
        let computed = schema_sha256(&self.scenario, self.schema, &self.symbols)?;
        if computed != self.schema_sha256 {
            return Err(LockError::HashMismatch {
                stored: self.schema_sha256.clone(),
                computed,
            });
        }
        Ok(())
    }

    /// Pretty JSON with sorted keys, 2-space indent and a trailing newline — the same layout as
    /// Python's `json.dumps(lock, indent=2, sort_keys=True, ensure_ascii=False) + "\n"`.
    pub fn to_json_pretty(&self) -> Result<String, LockError> {
        let mut s = serde_json::to_string_pretty(&sorted(serde_json::to_value(self)?))?;
        s.push('\n');
        Ok(s)
    }

    /// Derives the lock (symbols, kinds, roles, zone geometry, hash) from a manifest.
    pub fn from_manifest(
        m: &pb::Manifest,
        generator: impl Into<String>,
    ) -> Result<Self, LockError> {
        let ident = |s: &str| IdentSymbol {
            ident: s.to_owned(),
        };
        let kind_sym =
            |ident: &str, category: i32, roles: &[String]| -> Result<KindSymbol, LockError> {
                let kind = kind_of_category(category).ok_or_else(|| LockError::BadCategory {
                    ident: ident.to_owned(),
                    category,
                })?;
                Ok(KindSymbol {
                    ident: ident.to_owned(),
                    kind: kind.to_owned(),
                    roles: roles.to_vec(),
                })
            };
        let mut symbols = SymbolSet {
            airbases: m.airbases.iter().map(|a| ident(&a.ident)).collect(),
            flags: m.flags.iter().map(|f| ident(&f.ident)).collect(),
            groups: m
                .groups
                .iter()
                .filter(|g| !g.ident.is_empty())
                .map(|g| kind_sym(&g.ident, g.category, &g.roles))
                .collect::<Result<_, _>>()?,
            menus: m.menus.iter().map(|x| ident(&x.ident)).collect(),
            templates: m
                .templates
                .iter()
                .map(|t| kind_sym(&t.ident, t.category, &t.roles))
                .collect::<Result<_, _>>()?,
            zones: m.zones.iter().map(|z| ident(&z.ident)).collect(),
        };
        symbols.canonicalize();
        let zones = m
            .zones
            .iter()
            .filter_map(|z| {
                let geom = match z.shape.as_ref()? {
                    pb::zone::Shape::Circle(c) => ZoneGeometry::Circle {
                        center: c.center.map(MapPoint::from).unwrap_or_default(),
                        radius_m: c.radius_m,
                    },
                    pb::zone::Shape::Polygon(p) => ZoneGeometry::Polygon {
                        points: p.points.iter().copied().map(MapPoint::from).collect(),
                    },
                };
                Some((z.ident.clone(), geom))
            })
            .collect();
        let lock = ScenarioLock {
            format: LOCK_FORMAT,
            generator: generator.into(),
            geometry: GeometryLock { zones },
            scenario: m.scenario.clone(),
            schema: m.schema,
            schema_sha256: schema_sha256(&m.scenario, m.schema, &symbols)?,
            symbols,
        };
        lock.verify()?;
        Ok(lock)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbols() -> SymbolSet {
        SymbolSet {
            groups: vec![KindSymbol {
                ident: "SAM_NORTH".into(),
                kind: "ground".into(),
                roles: vec!["sam".into(), "north".into()],
            }],
            zones: vec![
                IdentSymbol {
                    ident: "SAM_RING".into(),
                },
                IdentSymbol {
                    ident: "OBJECTIVE".into(),
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn canonical_form() {
        let json = canonical_json("op_north", 3, &symbols()).unwrap();
        assert_eq!(
            std::str::from_utf8(&json).unwrap(),
            r#"{"scenario":"op_north","schema":3,"symbols":{"airbases":[],"flags":[],"groups":[{"ident":"SAM_NORTH","kind":"ground","roles":["north","sam"]}],"menus":[],"templates":[],"zones":[{"ident":"OBJECTIVE"},{"ident":"SAM_RING"}]}}"#
        );
    }

    #[test]
    fn checks() {
        let mut s = symbols();
        s.flags.push(IdentSymbol {
            ident: "bad".into(),
        });
        assert!(matches!(s.check(), Err(LockError::BadName { .. })));
        let mut s = symbols();
        s.groups[0].kind = "tank".into();
        assert!(matches!(s.check(), Err(LockError::BadKind { .. })));
        let mut s = symbols();
        s.zones.push(IdentSymbol {
            ident: "OBJECTIVE".into(),
        });
        assert!(matches!(s.check(), Err(LockError::Duplicate { .. })));
        assert!(check_scenario_name("op_north").is_ok());
        assert!(check_scenario_name("OpNorth").is_err());
    }
}
