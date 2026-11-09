//! `include_scenario!(op_north)`: includes `$OUT_DIR/sortie_scenario_op_north.rs`, written by
//! `sortie_build::scenario("op_north", lock).emit()` in `build.rs`, as `pub mod op_north`.
//!
//! Compared with a plain `include!` it explains the two usual mistakes at the macro call: no
//! build script (no `OUT_DIR`), and a name that differs from the one given to
//! `sortie_build::scenario` (it lists the scenarios the build script did generate).

use std::ffi::OsStr;
use std::path::Path;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token};

/// Must match `sortie_build::ScenarioBuild::out_file_name` (checked by a test).
pub(crate) const FILE_PREFIX: &str = "sortie_scenario_";

pub(crate) fn out_file_name(name: &str) -> String {
    format!("{FILE_PREFIX}{name}.rs")
}

struct Input {
    name: Ident,
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.is_empty() {
            return Err(input
                .error("expected the scenario name, e.g. `sortie::include_scenario!(op_north);`"));
        }
        let name: Ident = input.parse().map_err(|e| {
            syn::Error::new(
                e.span(),
                "expected the scenario name as a plain identifier, e.g. `op_north`",
            )
        })?;
        let _: Option<Token![,]> = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("include_scenario! takes only the scenario name"));
        }
        Ok(Self { name })
    }
}

pub(crate) fn expand(input: TokenStream, out_dir: Option<&OsStr>) -> syn::Result<TokenStream> {
    let Input { name } = syn::parse2(input)?;
    let text = name.to_string();
    if !is_scenario_name(&text) {
        return Err(syn::Error::new(
            name.span(),
            format!("`{text}` is not a scenario name: names match [a-z][a-z0-9_]*"),
        ));
    }
    let file = out_file_name(&text);
    let Some(out_dir) = out_dir else {
        return Err(syn::Error::new(
            name.span(),
            format!(
                "OUT_DIR is not set, so there is no generated scenario module to include. \
                 Add `sortie-build` to [build-dependencies] and a build.rs with \
                 `fn main() {{ sortie_build::scenario(\"{text}\", \"scenario.lock.json\").emit(); }}`"
            ),
        ));
    };
    let dir = Path::new(out_dir);
    if !dir.join(&file).is_file() {
        let found = generated_scenarios(dir);
        let hint = if found.is_empty() {
            format!(
                "build.rs did not generate any scenario; it needs \
                 `sortie_build::scenario(\"{text}\", \"<path to scenario.lock.json>\").emit()`"
            )
        } else {
            format!(
                "build.rs generated: {}; the name must be the first argument of \
                 `sortie_build::scenario(..)`",
                found.join(", ")
            )
        };
        return Err(syn::Error::new(
            name.span(),
            format!("no generated module for scenario `{text}` (OUT_DIR has no {file}); {hint}"),
        ));
    }
    let file = LitStr::new(&format!("/{file}"), Span::call_site());
    Ok(quote! {
        #[allow(dead_code, missing_docs, unreachable_pub, clippy::all)]
        pub mod #name {
            ::core::include!(::core::concat!(::core::env!("OUT_DIR"), #file));
        }
    })
}

/// `[a-z][a-z0-9_]*`, as `sortie_manifest::lock::check_scenario_name`.
fn is_scenario_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
}

/// Scenario names with a generated file in `dir`, sorted.
fn generated_scenarios(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| {
            let name = e.ok()?.file_name().into_string().ok()?;
            let stem = name.strip_prefix(FILE_PREFIX)?.strip_suffix(".rs")?;
            Some(stem.to_owned())
        })
        .collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(input: TokenStream, out_dir: Option<&OsStr>) -> String {
        expand(input, out_dir).unwrap_err().to_string()
    }

    #[test]
    fn the_file_name_matches_sortie_build() {
        let build = sortie_build::scenario("op_north", "scenario.lock.json");
        assert_eq!(out_file_name("op_north"), build.out_file_name());
    }

    #[test]
    fn includes_the_generated_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sortie_scenario_op_north.rs"), "").unwrap();
        let out = expand(quote!(op_north), Some(dir.path().as_os_str()))
            .unwrap()
            .to_string();
        assert!(out.contains("pub mod op_north"), "{out}");
        assert!(out.contains("\"/sortie_scenario_op_north.rs\""), "{out}");
        assert!(expand(quote!(op_north,), Some(dir.path().as_os_str())).is_ok());
    }

    #[test]
    fn explains_missing_build_scripts_and_wrong_names() {
        let msg = err(quote!(op_north), None);
        assert!(msg.contains("OUT_DIR is not set"), "{msg}");
        assert!(msg.contains("sortie_build::scenario(\"op_north\""), "{msg}");

        let dir = tempfile::tempdir().unwrap();
        let msg = err(quote!(op_north), Some(dir.path().as_os_str()));
        assert!(msg.contains("did not generate any scenario"), "{msg}");

        std::fs::write(dir.path().join("sortie_scenario_caucasus_2.rs"), "").unwrap();
        std::fs::write(dir.path().join("sortie_scenario_alpha.rs"), "").unwrap();
        std::fs::write(dir.path().join("other.rs"), "").unwrap();
        let msg = err(quote!(op_north), Some(dir.path().as_os_str()));
        assert!(msg.contains("generated: alpha, caucasus_2"), "{msg}");
    }

    #[test]
    fn rejects_bad_names_and_extra_input() {
        assert!(err(quote!(OpNorth), None).contains("not a scenario name"));
        assert!(err(quote!(r#type), None).contains("not a scenario name"));
        assert!(err(quote!(), None).contains("expected the scenario name"));
        assert!(err(quote!("op_north"), None).contains("plain identifier"));
        assert!(err(quote!(a, b), None).contains("only the scenario name"));
    }
}
