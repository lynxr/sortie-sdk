//! `#[sortie::test(manifest = "...")]`: an async test driven by a `TestHarness`. The harness
//! (FakeDcs, `FakeDcs<M>`) owns the runtime: `current_thread`, paused clock, `LocalSet`, and
//! loads the manifest. The macro only checks the shape and wires the call:
//!
//! ```ignore
//! #[test]
//! fn t() {
//!     async fn t(dcs: FakeDcs<OpNorth>) { ... }
//!     <FakeDcs<OpNorth> as ::sortie::testing::TestHarness>::run_test(
//!         Some(concat!(env!("CARGO_MANIFEST_DIR"), "/", "fixtures/op_north.manifest.pb")),
//!         t,
//!     )
//! }
//! ```
//!
//! The inner `async fn` carries the test's own name (it shadows the outer one inside its
//! body), so name-derived tooling such as insta snapshots sees `t`, not a generated name:
//! `assert_yaml_snapshot!` in `tests/sam.rs::t` writes `snapshots/sam__t.snap`.
//!
//! FakeDcs is not part of Sortie SDK 0.1: without the feature `fake-dcs` the attribute expands
//! to [`unavailable`], a compile error that says so.

use std::path::{Path, PathBuf};

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::meta::ParseNestedMeta;
use syn::parse::Parser;
use syn::spanned::Spanned;
use syn::{Expr, ExprLit, FnArg, Item, ItemFn, Lit, LitStr, ReturnType};

use crate::mission::{Errors, head_of, with_item};

const SHAPE: &str = "async fn my_test(dcs: FakeDcs<M>)";

/// The compile error of `#[sortie::test]` without the feature `fake-dcs` (the SDK).
pub(crate) const UNAVAILABLE: &str = "#[sortie::test] runs the logic against FakeDcs, which is \
    not part of Sortie SDK 0.1; tests without DCS arrive in SDK 0.2. Test against a live DCS \
    with `sortied --dev` for now.";

/// Manifest file extensions the loaders understand.
const EXTENSIONS: [&str; 2] = ["pb", "json"];

/// `#[sortie::test]` without FakeDcs: the error alone, at the attribute. A test function stays
/// defined as an empty item, so the error is not followed by unresolved names in its body.
pub(crate) fn unavailable(item: TokenStream) -> TokenStream {
    let mut out = quote!(::core::compile_error!(#UNAVAILABLE););
    if let Ok(Item::Fn(func)) = syn::parse2::<Item>(item) {
        let name = &func.sig.ident;
        out.extend(quote!(#[allow(dead_code)] fn #name() {}));
    }
    out
}

pub(crate) fn expand(
    attr: TokenStream,
    item: TokenStream,
    manifest_dir: Option<PathBuf>,
) -> TokenStream {
    let mut errors = Errors::default();
    let manifest = match parse_args(attr) {
        Ok(m) => m,
        Err(e) => {
            errors.push(e);
            None
        }
    };
    if let Some(lit) = &manifest
        && let Err(e) = check_manifest(lit, manifest_dir.as_deref())
    {
        errors.push(e);
    }
    let func = match syn::parse2::<Item>(item.clone()) {
        Ok(Item::Fn(func)) => Some(func),
        Ok(other) => {
            errors.push(syn::Error::new_spanned(
                head_of(&other),
                format!("#[sortie::test] goes on a test function: `{SHAPE}`"),
            ));
            None
        }
        Err(e) => {
            errors.push(e);
            None
        }
    };
    if let Some(func) = &func {
        check_signature(func, &mut errors);
    }
    match (errors.into_error(), func) {
        (None, Some(func)) => emit(&func, manifest.as_ref()),
        // Keep the user's function out of the output: an `async fn` next to a `#[test]`-like
        // attribute would only add confusing errors. An empty item keeps the name defined.
        (Some(err), Some(func)) => {
            let mut out = err.into_compile_error();
            let name = &func.sig.ident;
            out.extend(quote!(#[allow(dead_code)] fn #name() {}));
            out
        }
        (Some(err), None) => with_item(err, item),
        (None, None) => item,
    }
}

fn parse_args(attr: TokenStream) -> syn::Result<Option<LitStr>> {
    let mut manifest: Option<LitStr> = None;
    let parser = syn::meta::parser(|meta: ParseNestedMeta<'_>| {
        if meta.path.is_ident("manifest") {
            if manifest.is_some() {
                return Err(meta.error("`manifest` is given twice"));
            }
            let value: Expr = meta.value()?.parse()?;
            match value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) => {
                    manifest = Some(s);
                    Ok(())
                }
                other => Err(syn::Error::new_spanned(
                    other,
                    "`manifest` takes a string literal, e.g. \
                     `manifest = \"fixtures/op_north.manifest.pb\"`",
                )),
            }
        } else {
            Err(meta.error(
                "unknown #[sortie::test] argument; the only one is \
                 `manifest = \"path/relative/to/Cargo.toml/dir\"`",
            ))
        }
    });
    parser.parse2(attr)?;
    Ok(manifest)
}

/// Fails early (at the literal) for a path the harness could not load. Paths are relative to
/// the directory of the crate's `Cargo.toml`; absolute paths are used as they are.
fn check_manifest(lit: &LitStr, manifest_dir: Option<&Path>) -> syn::Result<()> {
    let value = lit.value();
    if value.trim().is_empty() {
        return Err(syn::Error::new_spanned(lit, "the manifest path is empty"));
    }
    let path = Path::new(&value);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !EXTENSIONS.contains(&ext) {
        return Err(syn::Error::new_spanned(
            lit,
            format!(
                "`{value}` is not a manifest: expected a `.pb` (protobuf) or `.json` file such \
                 as `fixtures/op_north.manifest.pb`"
            ),
        ));
    }
    let full = match manifest_dir {
        _ if path.is_absolute() => path.to_path_buf(),
        Some(dir) => dir.join(path),
        // No CARGO_MANIFEST_DIR (not built by cargo): leave the check to the harness.
        None => return Ok(()),
    };
    if full.is_file() {
        return Ok(());
    }
    let why = if full.exists() {
        "is not a file"
    } else {
        "does not exist"
    };
    Err(syn::Error::new_spanned(
        lit,
        format!(
            "manifest `{value}` {why}; the path is relative to the directory that holds this \
             crate's Cargo.toml"
        ),
    ))
}

fn check_signature(func: &ItemFn, errors: &mut Errors) {
    let sig = &func.sig;
    for attr in &func.attrs {
        let path = attr.path();
        if path.segments.last().is_some_and(|s| s.ident == "test") {
            errors.push(syn::Error::new_spanned(
                attr,
                "#[sortie::test] already makes this a test; remove this attribute",
            ));
        }
    }
    if sig.asyncness.is_none() {
        errors.push(syn::Error::new_spanned(
            sig.fn_token,
            format!("#[sortie::test] functions must be `async fn`: `{SHAPE}`"),
        ));
    }
    if !sig.generics.params.is_empty() || sig.generics.where_clause.is_some() {
        errors.push(syn::Error::new_spanned(
            &sig.generics,
            "#[sortie::test] functions are not generic; name the scenario, e.g. \
             `dcs: FakeDcs<OpNorth>`",
        ));
    }
    if let Some(c) = &sig.constness {
        errors.push(syn::Error::new_spanned(
            c,
            "#[sortie::test] functions cannot be `const`",
        ));
    }
    match sig.inputs.len() {
        1 => {}
        0 => errors.push(syn::Error::new(
            sig.paren_token.span.join(),
            format!("#[sortie::test] functions take the harness as their only argument: `{SHAPE}`"),
        )),
        n => errors.push(syn::Error::new_spanned(
            &sig.inputs,
            format!(
                "#[sortie::test] functions take exactly one argument, the harness \
                 (`dcs: FakeDcs<M>`); found {n}"
            ),
        )),
    }
    if let Some(FnArg::Receiver(r)) = sig.inputs.first() {
        errors.push(syn::Error::new_spanned(
            r,
            "#[sortie::test] functions are free functions, not methods",
        ));
    }
    if let ReturnType::Type(_, ty) = &sig.output {
        let unit = matches!(&**ty, syn::Type::Tuple(t) if t.elems.is_empty());
        if !unit {
            errors.push(syn::Error::new_spanned(
                ty,
                "#[sortie::test] functions return `()`; check results with `.unwrap()`, \
                 `.expect(..)` or assertions",
            ));
        }
    }
}

fn emit(func: &ItemFn, manifest: Option<&LitStr>) -> TokenStream {
    let Some(FnArg::Typed(arg)) = func.sig.inputs.first() else {
        return TokenStream::new();
    };
    let harness = &arg.ty;
    let name = &func.sig.ident;
    let attrs = &func.attrs;
    let vis = &func.vis;
    let inputs = &func.sig.inputs;
    let output = &func.sig.output;
    let body = &func.block;
    // Same name as the test: insta and panic messages name the test, not a generated helper.
    let body_fn = syn::Ident::new(&name.to_string(), name.span());
    let manifest = match manifest {
        Some(lit) if Path::new(&lit.value()).is_absolute() => {
            quote!(::core::option::Option::Some(#lit))
        }
        Some(lit) => quote!(::core::option::Option::Some(::core::concat!(
            ::core::env!("CARGO_MANIFEST_DIR"),
            "/",
            #lit
        ))),
        None => quote!(::core::option::Option::None),
    };
    let run_test = quote_spanned!(harness.span()=>
        <#harness as ::sortie::testing::TestHarness>::run_test
    );
    quote! {
        #[::core::prelude::v1::test]
        #(#attrs)*
        #vis fn #name() {
            async fn #body_fn(#inputs) #output #body
            #run_test(#manifest, #body_fn)
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;

    use super::*;

    fn dir() -> Option<PathBuf> {
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
    }

    fn errors(attr: TokenStream, item: TokenStream) -> String {
        let out = expand(attr, item, dir()).to_string();
        out.split("compile_error")
            .skip(1)
            .map(|s| s.split('"').nth(1).unwrap_or_default().to_owned())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn without_fake_dcs_it_is_one_clear_error() {
        let out = unavailable(quote! {
            #[should_panic]
            async fn sam_goes_red(dcs: FakeDcs<OpNorth>) { dcs.advance(1).await; }
        })
        .to_string();
        assert_eq!(out.matches("compile_error").count(), 1, "{out}");
        assert!(out.contains("not part of Sortie SDK 0.1"), "{out}");
        assert!(out.contains("tests without DCS arrive in SDK 0.2"), "{out}");
        assert!(out.contains("fn sam_goes_red () { }"), "{out}");
        assert!(!out.contains("FakeDcs <"), "{out}");
        // Not a function: still the one error.
        let out = unavailable(quote!(
            struct S;
        ))
        .to_string();
        assert_eq!(out.matches("compile_error").count(), 1, "{out}");
    }

    #[test]
    fn expands_to_a_plain_test_calling_the_harness() {
        let out = expand(
            quote!(manifest = "Cargo.toml.json"),
            quote! { async fn t(dcs: FakeDcs<OpNorth>) { dcs.advance(1).await; } },
            dir(),
        )
        .to_string();
        // Cargo.toml.json does not exist: the literal is reported.
        assert!(out.contains("does not exist"), "{out}");

        let out = expand(
            TokenStream::new(),
            quote! {
                #[should_panic]
                async fn t(dcs: FakeDcs<OpNorth>) { dcs.advance(1).await; }
            },
            dir(),
        )
        .to_string();
        assert!(!out.contains("compile_error"), "{out}");
        assert!(
            out.starts_with("# [:: core :: prelude :: v1 :: test] # [should_panic] fn t ()"),
            "{out}"
        );
        assert!(
            out.contains(
                "< FakeDcs < OpNorth > as :: sortie :: testing :: TestHarness > :: run_test \
                 (:: core :: option :: Option :: None , t)"
            ),
            "{out}"
        );
    }

    #[test]
    fn manifest_paths_are_checked_relative_to_the_crate() {
        let manifest = "../../examples/op_north/logic/fixtures/op_north.manifest.pb";
        let lit = LitStr::new(manifest, Span::call_site());
        assert!(check_manifest(&lit, dir().as_deref()).is_ok());
        let out = expand(
            quote!(manifest = #lit),
            quote! { async fn t(dcs: FakeDcs<OpNorth>) {} },
            dir(),
        )
        .to_string();
        assert!(!out.contains("compile_error"), "{out}");
        assert!(out.contains("CARGO_MANIFEST_DIR"), "{out}");

        let lit = LitStr::new("fixtures/missing.manifest.pb", Span::call_site());
        let err = check_manifest(&lit, dir().as_deref())
            .unwrap_err()
            .to_string();
        assert!(err.contains("does not exist"), "{err}");
        assert!(err.contains("relative to the directory"), "{err}");
        // Without CARGO_MANIFEST_DIR the harness reports a missing file at run time.
        assert!(check_manifest(&lit, None).is_ok());

        let lit = LitStr::new("src", Span::call_site());
        let err = check_manifest(&lit, dir().as_deref())
            .unwrap_err()
            .to_string();
        assert!(err.contains("not a manifest"), "{err}");
        let lit = LitStr::new("src.json", Span::call_site());
        assert!(check_manifest(&lit, Some(Path::new("/"))).is_err());
        let lit = LitStr::new(" ", Span::call_site());
        assert!(
            check_manifest(&lit, None)
                .unwrap_err()
                .to_string()
                .contains("empty")
        );
    }

    #[test]
    fn bad_arguments_and_signatures_are_reported_together() {
        let errs = errors(
            quote!(manifest = "a.pb", manifest = "b.pb"),
            quote! { fn t() -> u32 { 1 } },
        );
        for needle in [
            "`manifest` is given twice",
            "must be `async fn`",
            "only argument",
            "return `()`",
        ] {
            assert!(errs.contains(needle), "missing {needle:?} in\n{errs}");
        }
        assert!(errors(quote!(path = "x"), quote! { async fn t(d: D) {} }).contains("unknown"));
        assert!(errors(quote!(manifest = 1), quote! { async fn t(d: D) {} }).contains("string"));
        let errs = errors(
            TokenStream::new(),
            quote! { #[tokio::test] async fn t<T>(a: A, b: B) {} },
        );
        assert!(errs.contains("already makes this a test"), "{errs}");
        assert!(errs.contains("not generic"), "{errs}");
        assert!(errs.contains("found 2"), "{errs}");
    }

    #[test]
    fn an_absolute_manifest_path_is_used_as_is() {
        let abs = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/op_north/logic/fixtures/op_north.manifest.pb");
        let lit = LitStr::new(abs.to_str().unwrap(), Span::call_site());
        let out = expand(
            quote!(manifest = #lit),
            quote! { async fn t(dcs: FakeDcs<OpNorth>) {} },
            None,
        )
        .to_string();
        assert!(!out.contains("CARGO_MANIFEST_DIR"), "{out}");
        assert!(!out.contains("compile_error"), "{out}");
    }
}
