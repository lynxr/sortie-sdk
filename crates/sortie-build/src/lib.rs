//! `build.rs` helper that turns `scenario.lock.json` into typed scenario handles.
//!
//! ```ignore
//! // logic/build.rs
//! fn main() { sortie_build::scenario("op_north", "../scenario.lock.json").emit(); }
//! ```
//!
//! and in the logic crate:
//!
//! ```ignore
//! sortie::include_scenario!(op_north);
//! use op_north::{groups, zones, OpNorth};
//! ```
//!
//! # Output
//!
//! `emit()` writes `$OUT_DIR/sortie_scenario_<name>.rs`, which `sortie::include_scenario!(<name>)`
//! includes inside `pub mod <name> { ... }`. The file contains:
//!
//! * `pub struct <CamelName>;` implementing `::sortie::types::Scenario`;
//! * `pub mod groups` — `pub const <IDENT>: GroupRef<<CamelName>, <Kind>>` per group symbol;
//! * `pub mod zones` — `pub const <IDENT>: ZoneRef<<CamelName>>` with the lock's const geometry
//!   (`ZoneRef::circle(..)` or `ZoneRef::polygon(..)`), so `zones::X.center()` works in tests;
//! * `pub mod templates` — `TemplateRef<<CamelName>, <Kind>>`;
//! * `pub mod flags`, `pub mod airbases`, `pub mod menus` — `FlagRef`, `AirbaseRef`, `MenuRef`.
//!
//! Every module is always present (possibly empty). Paths in generated code start with
//! `::sortie::types` (configurable with [`ScenarioBuild::sortie_path`], or
//! [`ScenarioBuild::types_path`] for crates that depend on `sortie-types` directly).

use std::path::{Path, PathBuf};

use heck::ToUpperCamelCase;
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use sortie_manifest::{IdentSymbol, KindSymbol, LockError, ScenarioLock, ZoneGeometry};

/// Errors of the code generator.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("{path}: {source}")]
    Lock {
        path: String,
        #[source]
        source: LockError,
    },
    #[error("{path} is for scenario {found:?}, but build.rs asked for {expected:?}")]
    WrongScenario {
        path: String,
        expected: String,
        found: String,
    },
    #[error(
        "scenario name {0:?} is a Rust keyword and cannot name the generated module; rename the scenario"
    )]
    Keyword(String),
    #[error("zone {0:?} has no geometry in scenario.lock.json (geometry.zones)")]
    MissingGeometry(String),
    #[error("zone {ident:?} has invalid geometry in scenario.lock.json: {reason}")]
    BadGeometry { ident: String, reason: String },
    #[error("cannot write {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("environment variable {0} is not set (emit() must run inside a build script)")]
    Env(&'static str),
    #[error("internal error: generated code does not parse: {0}")]
    Syntax(String),
}

impl BuildError {
    /// A hint for the person reading a failed build.
    #[must_use]
    pub fn help(&self) -> &'static str {
        match self {
            BuildError::Lock {
                source: LockError::Io { .. },
                ..
            } => {
                "the path is relative to the logic crate's Cargo.toml; create the lock with `sortie-gen lock` (python scenario.py lock)"
            }
            BuildError::Lock {
                source: LockError::HashMismatch { .. },
                ..
            } => {
                "the lock was edited by hand or is corrupt; regenerate it with `sortie-gen lock` instead of editing it"
            }
            BuildError::Lock { .. }
            | BuildError::MissingGeometry(_)
            | BuildError::BadGeometry { .. } => {
                "regenerate the lock with `sortie-gen lock` (python scenario.py lock)"
            }
            BuildError::WrongScenario { .. } => {
                "the first argument of sortie_build::scenario(...) must be the lock's \"scenario\""
            }
            BuildError::Keyword(_) => {
                "scenario names are [a-z][a-z0-9_]* and must not be Rust keywords"
            }
            BuildError::Io { .. } | BuildError::Env(_) => "this is a build environment problem",
            BuildError::Syntax(_) => {
                "this is a bug in sortie-build; please report it with the lock file"
            }
        }
    }
}

/// Rust keywords (strict and reserved, edition 2024) that cannot name a module.
const KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Is `name` usable as the generated module name (`include_scenario!(name)`)?
#[must_use]
pub fn is_keyword(name: &str) -> bool {
    KEYWORDS.contains(&name)
}

/// Starts a build for scenario `name` from the lock file at `lock_path` (relative paths are
/// resolved against `CARGO_MANIFEST_DIR`).
pub fn scenario(name: &str, lock_path: impl AsRef<Path>) -> ScenarioBuild {
    ScenarioBuild {
        name: name.to_owned(),
        lock_path: lock_path.as_ref().to_path_buf(),
        types_path: "::sortie::types".into(),
    }
}

/// Builder returned by [`scenario`].
#[derive(Clone, Debug)]
pub struct ScenarioBuild {
    name: String,
    lock_path: PathBuf,
    /// Path of the `sortie_types` crate as seen from the generated code.
    types_path: String,
}

impl ScenarioBuild {
    /// Path of the SDK crate in generated code (default `::sortie`); handles are referenced as
    /// `<path>::types::GroupRef` etc.
    #[must_use]
    pub fn sortie_path(mut self, path: &str) -> Self {
        self.types_path = format!("{path}::types");
        self
    }

    /// Path of the `sortie-types` crate itself in generated code, e.g. `::sortie_types` for a
    /// crate that does not depend on `sortie`.
    #[must_use]
    pub fn types_path(mut self, path: &str) -> Self {
        self.types_path = path.to_owned();
        self
    }

    /// File name under `OUT_DIR` that `include_scenario!(<name>)` includes.
    #[must_use]
    pub fn out_file_name(&self) -> String {
        format!("sortie_scenario_{}.rs", self.name)
    }

    fn resolved_lock_path(&self) -> PathBuf {
        if self.lock_path.is_absolute() {
            return self.lock_path.clone();
        }
        match std::env::var_os("CARGO_MANIFEST_DIR") {
            Some(dir) => PathBuf::from(dir).join(&self.lock_path),
            None => self.lock_path.clone(),
        }
    }

    /// Reads and verifies the lock file and returns the generated Rust source.
    pub fn generate(&self) -> Result<String, BuildError> {
        let path = self.resolved_lock_path();
        let lock = ScenarioLock::from_path(&path).map_err(|source| BuildError::Lock {
            path: path.display().to_string(),
            source,
        })?;
        if lock.scenario != self.name {
            return Err(BuildError::WrongScenario {
                path: path.display().to_string(),
                expected: self.name.clone(),
                found: lock.scenario,
            });
        }
        generate_with_types_path(&lock, &self.types_path)
    }

    /// Generates into `OUT_DIR` and returns the written path. The file is only rewritten when
    /// its content changes, so an unchanged lock does not rebuild the logic crate.
    pub fn try_emit(&self) -> Result<PathBuf, BuildError> {
        let lock_path = self.resolved_lock_path();
        println!("cargo:rerun-if-changed={}", lock_path.display());
        let out_dir = std::env::var_os("OUT_DIR").ok_or(BuildError::Env("OUT_DIR"))?;
        self.write_to(Path::new(&out_dir))
    }

    /// Generates `<dir>/sortie_scenario_<name>.rs` (rewritten only when the content changes).
    pub fn write_to(&self, dir: &Path) -> Result<PathBuf, BuildError> {
        let source = self.generate()?;
        let out = dir.join(self.out_file_name());
        if std::fs::read_to_string(&out).ok().as_deref() != Some(source.as_str()) {
            std::fs::write(&out, source).map_err(|source| BuildError::Io {
                path: out.display().to_string(),
                source,
            })?;
        }
        Ok(out)
    }

    /// [`ScenarioBuild::try_emit`] for build scripts: on error prints a readable message and
    /// exits with status 1 (no panic backtrace).
    pub fn emit(self) {
        if let Err(e) = self.try_emit() {
            eprintln!(
                "error: sortie-build failed for scenario {:?}: {e}",
                self.name
            );
            eprintln!("help: {}", e.help());
            std::process::exit(1);
        }
    }
}

/// Rust type name of a scenario marker: `op_north` -> `OpNorth`.
#[must_use]
pub fn marker_name(scenario: &str) -> String {
    scenario.to_upper_camel_case()
}

fn path_tokens(path: &str) -> Result<TokenStream, BuildError> {
    syn::parse_str::<syn::Path>(path)
        .map(|p| quote!(#p))
        .map_err(|e| BuildError::Syntax(e.to_string()))
}

fn f64_tokens(v: f64) -> TokenStream {
    let lit = Literal::f64_unsuffixed(v.abs());
    if v.is_sign_negative() {
        quote!(-#lit)
    } else {
        quote!(#lit)
    }
}

fn kind_tokens(types: &TokenStream, kind: &str) -> TokenStream {
    match kind {
        "air" => quote!(#types::Air),
        "helo" => quote!(#types::Helo),
        "ship" => quote!(#types::Ship),
        _ => quote!(#types::Ground),
    }
}

fn roles_doc(s: &KindSymbol) -> String {
    if s.roles.is_empty() {
        format!(" `{}` ({}).", s.ident, s.kind)
    } else {
        format!(
            " `{}` ({}; roles: {}).",
            s.ident,
            s.kind,
            s.roles.join(", ")
        )
    }
}

/// Const geometry must be finite (a NaN literal does not even parse) and a real shape.
fn check_geometry(ident: &str, geom: &ZoneGeometry) -> Result<(), BuildError> {
    let bad = |reason: &str| BuildError::BadGeometry {
        ident: ident.to_owned(),
        reason: reason.to_owned(),
    };
    let shape = match geom {
        ZoneGeometry::Circle { center, radius_m } => sortie_types::ZoneShape::Circle {
            center: *center,
            radius_m: *radius_m,
        },
        ZoneGeometry::Polygon { points } => sortie_types::ZoneShape::Polygon { points },
    };
    if shape.is_valid() {
        Ok(())
    } else {
        Err(bad(match geom {
            ZoneGeometry::Circle { .. } => "a circle needs a finite centre and a finite radius > 0",
            ZoneGeometry::Polygon { .. } => {
                "a polygon needs at least 3 finite points and a non-zero area"
            }
        }))
    }
}

/// Generates the scenario module body for a verified lock; handles are referenced as
/// `<sortie_path>::types::*`.
pub fn generate(lock: &ScenarioLock, sortie_path: &str) -> Result<String, BuildError> {
    generate_with_types_path(lock, &format!("{sortie_path}::types"))
}

/// [`generate`] with the path of the `sortie-types` crate given directly.
pub fn generate_with_types_path(
    lock: &ScenarioLock,
    types_path: &str,
) -> Result<String, BuildError> {
    if is_keyword(&lock.scenario) {
        return Err(BuildError::Keyword(lock.scenario.clone()));
    }
    let types = path_tokens(types_path)?;
    let marker = format_ident!("{}", marker_name(&lock.scenario));
    let name = &lock.scenario;
    let schema = Literal::u32_unsuffixed(lock.schema);
    let sha = &lock.schema_sha256;

    let mut symbols = Vec::new();
    let mut push_symbols = |kind: &str, list: &mut dyn Iterator<Item = &str>| {
        let k = format_ident!("{}", kind);
        for ident in list {
            symbols.push(quote!(#types::Symbol::new(#types::SymbolKind::#k, #ident)));
        }
    };
    let s = &lock.symbols;
    push_symbols("Group", &mut s.groups.iter().map(|x| x.ident.as_str()));
    push_symbols("Zone", &mut s.zones.iter().map(|x| x.ident.as_str()));
    push_symbols(
        "Template",
        &mut s.templates.iter().map(|x| x.ident.as_str()),
    );
    push_symbols("Flag", &mut s.flags.iter().map(|x| x.ident.as_str()));
    push_symbols("Airbase", &mut s.airbases.iter().map(|x| x.ident.as_str()));
    push_symbols("Menu", &mut s.menus.iter().map(|x| x.ident.as_str()));

    let groups = s.groups.iter().map(|g| {
        let id = format_ident!("{}", g.ident);
        let ident = &g.ident;
        let kind = kind_tokens(&types, &g.kind);
        let doc = format!(" Group{}", roles_doc(g));
        quote! {
            #[doc = #doc]
            pub const #id: #types::GroupRef<super::#marker, #kind> = #types::GroupRef::new(#ident);
        }
    });
    let templates = s.templates.iter().map(|t| {
        let id = format_ident!("{}", t.ident);
        let ident = &t.ident;
        let kind = kind_tokens(&types, &t.kind);
        let doc = format!(" Template{}", roles_doc(t));
        quote! {
            #[doc = #doc]
            pub const #id: #types::TemplateRef<super::#marker, #kind> = #types::TemplateRef::new(#ident);
        }
    });
    let zones = s
        .zones
        .iter()
        .map(|z| {
            let id = format_ident!("{}", z.ident);
            let ident = &z.ident;
            let geom = lock
                .geometry
                .zones
                .get(&z.ident)
                .ok_or_else(|| BuildError::MissingGeometry(z.ident.clone()))?;
            check_geometry(&z.ident, geom)?;
            let point = |p: &sortie_types::MapPoint| {
                let (n, e) = (f64_tokens(p.north), f64_tokens(p.east));
                quote!(#types::MapPoint::new(#n, #e))
            };
            let (value, doc) = match geom {
                ZoneGeometry::Circle { center, radius_m } => {
                    let c = point(center);
                    let r = f64_tokens(*radius_m);
                    (
                        quote!(#types::ZoneRef::circle(#ident, #c, #r)),
                        format!(" Zone `{ident}`: circle, r = {radius_m} m."),
                    )
                }
                ZoneGeometry::Polygon { points } => {
                    let pts = points.iter().map(point);
                    (
                        quote!(#types::ZoneRef::polygon(#ident, &[#(#pts),*])),
                        format!(" Zone `{ident}`: polygon, {} points.", points.len()),
                    )
                }
            };
            Ok(quote! {
                #[doc = #doc]
                pub const #id: #types::ZoneRef<super::#marker> = #value;
            })
        })
        .collect::<Result<Vec<_>, BuildError>>()?;
    let simple = |list: &[IdentSymbol], ty: &str, what: &str| -> Vec<TokenStream> {
        let ty = format_ident!("{}", ty);
        list.iter()
            .map(|x| {
                let id = format_ident!("{}", x.ident);
                let ident = &x.ident;
                let doc = format!(" {what} `{ident}`.");
                quote! {
                    #[doc = #doc]
                    pub const #id: #types::#ty<super::#marker> = #types::#ty::new(#ident);
                }
            })
            .collect()
    };
    let flags = simple(&s.flags, "FlagRef", "Flag");
    let airbases = simple(&s.airbases, "AirbaseRef", "Airbase");
    let menus = simple(&s.menus, "MenuRef", "Menu");

    let marker_doc = format!(" Scenario marker for `{name}` (schema {}).", lock.schema);
    let tokens = quote! {
        #[doc = #marker_doc]
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub struct #marker;

        impl #types::Scenario for #marker {
            const NAME: &'static str = #name;
            const SCHEMA: u32 = #schema;
            const SCHEMA_SHA256: &'static str = #sha;
            const SYMBOLS: &'static [#types::Symbol] = &[#(#symbols),*];
        }

        /// Groups placed in the mission.
        pub mod groups { #(#groups)* }
        /// Trigger zones (geometry from scenario.lock.json; the running manifest wins at run time).
        pub mod zones { #(#zones)* }
        /// Spawnable templates (kept out of the .miz).
        pub mod templates { #(#templates)* }
        /// User flags.
        pub mod flags { #(#flags)* }
        /// Airbases.
        pub mod airbases { #(#airbases)* }
        /// F10 radio menu commands.
        pub mod menus { #(#menus)* }
    };
    let file: syn::File = syn::parse2(tokens).map_err(|e| BuildError::Syntax(e.to_string()))?;
    let header = format!(
        "// @generated by sortie-build {} from scenario.lock.json (scenario {name:?}, schema {}).\n// Do not edit: regenerate the lock with sortie-gen instead.\n\n",
        env!("CARGO_PKG_VERSION"),
        lock.schema
    );
    Ok(header + &prettyplease::unparse(&file))
}
