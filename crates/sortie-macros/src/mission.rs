//! `#[sortie::mission]`: checks the shape of a mission's entry point and emits it unchanged.
//!
//! Two layers of checks:
//!
//! * syntax (here; all errors at once, in Sortie's own words, so the trybuild tests in
//!   `tests/ui` do not depend on rustc's wording): `async fn`, two plain arguments, an explicit
//!   return type, no generics, receivers, `const`/`unsafe`/`extern`, variadics or
//!   `impl Trait` / reference arguments. Types spelled with the SDK's names (the last path
//!   segment) are checked too: `Ctx` comes first with one type argument, `Start` second with
//!   none, and a `Result` return type is `Result<()>` or `Result<(), ..Error>`. A return type
//!   that is `()`, a tuple, `!`, a reference, slice, array, pointer, `fn`, `dyn` or
//!   `impl Trait` is rejected. Any other name (an alias such as `type R = sortie::Result<()>`,
//!   a renamed import) is left to the compiler;
//! * types (by the compiler, once the syntax checks passed): a hidden `const _` block passes
//!   the declared argument and return types to functions that only accept
//!   `::sortie::Ctx<M: Scenario>`, `::sortie::Start` and `::sortie::Result<()>`, spanned at the
//!   user's types, so a wrong type (a marker that is not a scenario, an alias of another type)
//!   is reported at the definition instead of deep inside `sortie::main!` / `FakeDcs::run`
//!   (whose bound is `sortie::runtime::Mission<M>`). These errors are rustc's, so they are
//!   pinned by error code only, in the `compile_fail` doctests of `sortie`
//!   (`crates/sortie/src/mission_checks.rs`).

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use quote::{ToTokens, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    FnArg, GenericArgument, Item, ItemFn, PatType, PathArguments, PathSegment, ReturnType, Safety,
    Signature, Type,
};

pub(crate) const SHAPE: &str = "async fn run(ctx: Ctx<M>, start: Start) -> sortie::Result<()>";

/// The expected order, for the messages about misplaced arguments.
const ORDER: &str = "a mission takes `ctx: Ctx<M>` first, then `start: Start`";

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = match syn::parse2::<Item>(item.clone()) {
        Ok(Item::Fn(func)) => func,
        Ok(other) => {
            let err = syn::Error::new_spanned(
                head_of(&other),
                format!("#[sortie::mission] goes on the mission's entry point: `{SHAPE}`"),
            );
            return with_item(err, item);
        }
        Err(err) => return with_item(err, item),
    };
    let mut errors = Errors::default();
    if !attr.is_empty() {
        errors.push(syn::Error::new_spanned(
            &attr,
            "#[sortie::mission] takes no arguments",
        ));
    }
    check_signature(&func, &mut errors);
    match errors.into_error() {
        Some(err) => with_item(err, func.to_token_stream()),
        None => {
            let checks = type_checks(&func);
            quote! {
                #func
                #checks
            }
        }
    }
}

fn check_signature(func: &ItemFn, errors: &mut Errors) {
    let sig = &func.sig;
    if let Some(c) = &sig.constness {
        errors.push(syn::Error::new_spanned(c, "a mission cannot be `const`"));
    }
    if let Safety::Unsafe(u) = &sig.safety {
        errors.push(syn::Error::new_spanned(u, "a mission cannot be `unsafe`"));
    }
    if let Some(abi) = &sig.abi {
        errors.push(syn::Error::new_spanned(
            abi,
            "a mission is a plain Rust function; remove the `extern` ABI",
        ));
    }
    if sig.asyncness.is_none() {
        errors.push(syn::Error::new_spanned(
            sig.fn_token,
            format!("a mission must be an `async fn`: `{SHAPE}`"),
        ));
    }
    if !sig.generics.params.is_empty() {
        errors.push(syn::Error::new_spanned(
            &sig.generics,
            "a mission is not generic: name the scenario in the argument type, e.g. \
             `ctx: Ctx<op_north::OpNorth>`",
        ));
    }
    if let Some(w) = &sig.generics.where_clause {
        errors.push(syn::Error::new_spanned(
            w,
            "a mission is not generic; remove the `where` clause",
        ));
    }
    if let Some(v) = &sig.variadic {
        errors.push(syn::Error::new_spanned(v, "a mission is not variadic"));
    }

    let mut typed = 0usize;
    for arg in &sig.inputs {
        match arg {
            FnArg::Receiver(r) => errors.push(syn::Error::new_spanned(
                r,
                "a mission is a free function, not a method",
            )),
            FnArg::Typed(pt) => {
                let what = match typed {
                    0 => "the context, `Ctx<M>`",
                    _ => "the start reason, `Start`",
                };
                typed += 1;
                match &*pt.ty {
                    Type::ImplTrait(t) => errors.push(syn::Error::new_spanned(
                        t,
                        format!("`impl Trait` makes the mission generic; this argument is {what}"),
                    )),
                    Type::Reference(r) => errors.push(syn::Error::new_spanned(
                        r,
                        format!(
                            "a mission takes its arguments by value (the context is cheap to \
                             clone); this argument is {what}"
                        ),
                    )),
                    _ => {}
                }
            }
        }
    }
    check_argument_types(sig, errors);
    let count = sig.inputs.len();
    if count != 2 {
        let msg = format!(
            "a mission takes exactly two arguments, `ctx: Ctx<M>` and `start: Start`; found {count}"
        );
        let err = if sig.inputs.is_empty() {
            syn::Error::new(sig.paren_token.span.join(), msg)
        } else {
            syn::Error::new_spanned(&sig.inputs, msg)
        };
        errors.push(err);
    }
    match &sig.output {
        ReturnType::Default => errors.push(syn::Error::new_spanned(
            signature_head(func),
            "a mission must declare its return type: `-> sortie::Result<()>`",
        )),
        ReturnType::Type(_, ty) => {
            if !may_be_mission_result(ty) {
                errors.push(syn::Error::new_spanned(
                    ty,
                    format!(
                        "a mission must return `sortie::Result<()>`; found `{}`",
                        written(ty)
                    ),
                ));
            }
        }
    }
}

/// The order of `Ctx<M>` and `Start` and their generic arguments, for arguments spelled with
/// those names. Other names are left to [`type_checks`].
fn check_argument_types(sig: &Signature, errors: &mut Errors) {
    let typed: Vec<&PatType> = sig
        .inputs
        .iter()
        .filter_map(|a| match a {
            FnArg::Typed(pt) => Some(pt),
            FnArg::Receiver(_) => None,
        })
        .collect();
    let first = typed.first().copied();
    let second = typed.get(1).copied();
    let start_first = first.filter(|a| sdk_name(&a.ty, "Start").is_some());
    let ctx_second = second.filter(|a| sdk_name(&a.ty, "Ctx").is_some());
    match (start_first, ctx_second) {
        (Some(s), Some(c)) => errors.push(syn::Error::new_spanned(
            quote!(#s #c),
            format!("the arguments are swapped: {ORDER}"),
        )),
        (Some(s), None) => errors.push(syn::Error::new_spanned(
            &s.ty,
            format!("`Start` is the second argument: {ORDER}"),
        )),
        (None, Some(c)) => errors.push(syn::Error::new_spanned(
            &c.ty,
            format!("`Ctx` is the first argument: {ORDER}"),
        )),
        (None, None) => {}
    }
    if let Some(a) = first
        && let Some(args) = sdk_name(&a.ty, "Ctx")
        && !is_one_type(args)
    {
        errors.push(syn::Error::new_spanned(
            &a.ty,
            format!(
                "the context takes the scenario marker as its only type argument: \
                 `ctx: Ctx<M>`, e.g. `ctx: Ctx<op_north::OpNorth>`; found `{}`",
                written(&a.ty)
            ),
        ));
    }
    if let Some(a) = second
        && let Some(args) = sdk_name(&a.ty, "Start")
        && !args.is_none()
    {
        errors.push(syn::Error::new_spanned(
            &a.ty,
            format!(
                "`Start` takes no generic arguments: `start: Start`; found `{}`",
                written(&a.ty)
            ),
        ));
    }
}

/// The generic arguments of `ty` when it is a plain path whose last segment is `name`.
fn sdk_name<'a>(ty: &'a Type, name: &str) -> Option<&'a PathArguments> {
    last_segment(ty)
        .filter(|s| s.ident == name)
        .map(|s| &s.arguments)
}

/// The last segment of a plain path (no `<T as Trait>::` qualifier), through parentheses and
/// the invisible groups of `macro_rules` fragments.
fn last_segment(ty: &Type) -> Option<&PathSegment> {
    match ty {
        Type::Path(p) if p.qself.is_none() => p.path.segments.last(),
        Type::Group(g) => last_segment(&g.elem),
        Type::Paren(p) => last_segment(&p.elem),
        _ => None,
    }
}

/// `<T>` with one type, which names something (not `_`).
fn is_one_type(args: &PathArguments) -> bool {
    let PathArguments::AngleBracketed(a) = args else {
        return false;
    };
    a.args.len() == 1
        && matches!(a.args.first(), Some(GenericArgument::Type(t)) if !matches!(t, Type::Infer(_)))
}

/// `false` for a return type that cannot be `sortie::Result<()>`. `sortie::Result<T, E = Error>`
/// is `core::result::Result<T, E>`, so a type spelled `Result` must be `Result<()>` or
/// `Result<(), E>` with `E` named `..Error` (e.g. `sortie::Error`, or `SortieError` imported
/// under another name); whether it really is the SDK's type is up to [`type_checks`]. So is
/// any other name, which may be an alias.
fn may_be_mission_result(ty: &Type) -> bool {
    match ty {
        Type::Group(g) => may_be_mission_result(&g.elem),
        Type::Paren(p) => may_be_mission_result(&p.elem),
        Type::Path(p) if p.qself.is_none() => match p.path.segments.last() {
            Some(seg) if seg.ident == "Result" => is_result_of_unit(&seg.arguments),
            _ => true,
        },
        Type::Tuple(_)
        | Type::Never(_)
        | Type::Reference(_)
        | Type::Slice(_)
        | Type::Array(_)
        | Type::Ptr(_)
        | Type::FnPtr(_)
        | Type::TraitObject(_)
        | Type::ImplTrait(_)
        | Type::Infer(_) => false,
        _ => true,
    }
}

/// `<()>` or `<(), ..Error>`.
fn is_result_of_unit(args: &PathArguments) -> bool {
    let PathArguments::AngleBracketed(a) = args else {
        return false;
    };
    let mut it = a.args.iter();
    match (it.next(), it.next(), it.next()) {
        (Some(GenericArgument::Type(ok)), None, None) => is_unit(ok),
        (Some(GenericArgument::Type(ok)), Some(GenericArgument::Type(err)), None) => {
            is_unit(ok) && last_segment(err).is_some_and(|s| s.ident.to_string().ends_with("Error"))
        }
        _ => false,
    }
}

fn is_unit(ty: &Type) -> bool {
    match ty {
        Type::Tuple(t) => t.elems.is_empty(),
        Type::Group(g) => is_unit(&g.elem),
        Type::Paren(p) => is_unit(&p.elem),
        _ => false,
    }
}

/// A type as written, for "found `..`": the tokens without the spaces `to_string` puts between
/// all of them, so `Result<(), String>` reads as in the source.
fn written(tokens: &impl ToTokens) -> String {
    let mut out = String::new();
    write_tokens(tokens.to_token_stream(), &mut out);
    out
}

fn write_tokens(tokens: TokenStream, out: &mut String) {
    /// What the last token was, for the space before a word or a group.
    #[derive(PartialEq)]
    enum Prev {
        /// A keyword that a type follows: `&mut (u8, u8)`, `*const [u8]`, `dyn (Trait)`.
        Keyword,
        /// The end of a name: an ident, a literal, `>`.
        Word,
        Other,
    }
    let mut prev = Prev::Other;
    let mut iter = tokens.into_iter().peekable();
    while let Some(tt) = iter.next() {
        match tt {
            TokenTree::Ident(_) | TokenTree::Literal(_) => {
                if prev != Prev::Other {
                    out.push(' ');
                }
                let word = tt.to_string();
                out.push_str(&word);
                prev = match word.as_str() {
                    "const" | "dyn" | "impl" | "mut" => Prev::Keyword,
                    _ => Prev::Word,
                };
            }
            TokenTree::Group(g) => {
                let (open, close, space) = match g.delimiter() {
                    // `fn(u32)`, `Fn()`: no space after a name.
                    Delimiter::Parenthesis => ("(", ")", prev == Prev::Keyword),
                    Delimiter::Bracket => ("[", "]", prev != Prev::Other),
                    Delimiter::Brace => ("{ ", " }", prev != Prev::Other),
                    Delimiter::None => ("", "", prev != Prev::Other),
                };
                if space {
                    out.push(' ');
                }
                out.push_str(open);
                write_tokens(g.stream(), out);
                out.push_str(close);
                prev = if g.delimiter() == Delimiter::None {
                    Prev::Word
                } else {
                    Prev::Other
                };
            }
            TokenTree::Punct(p) => {
                let c = p.as_char();
                let (last, next_punct) = match iter.peek() {
                    None => (true, None),
                    Some(TokenTree::Punct(n)) => (false, Some(n.as_char())),
                    Some(_) => (false, None),
                };
                prev = Prev::Other;
                match c {
                    // No space before the end of the type or of `<..>`: `Result<(),>`.
                    ',' | ';' => {
                        out.push(c);
                        if !last && next_punct != Some('>') {
                            out.push(' ');
                        }
                    }
                    '+' | '=' => {
                        out.push(' ');
                        out.push(c);
                        out.push(' ');
                    }
                    '-' if p.spacing() == Spacing::Joint && next_punct == Some('>') => {
                        iter.next();
                        out.push_str(" -> ");
                    }
                    '>' => {
                        out.push(c);
                        prev = Prev::Word;
                    }
                    // A lifetime is `'` joined to an ident: no space between them.
                    _ => out.push(c),
                }
            }
        }
    }
}

/// The hidden type assertions; only called once the syntax checks passed.
fn type_checks(func: &ItemFn) -> TokenStream {
    let tys: Vec<&Type> = func
        .sig
        .inputs
        .iter()
        .filter_map(|a| match a {
            FnArg::Typed(pt) => Some(&*pt.ty),
            FnArg::Receiver(_) => None,
        })
        .collect();
    let (Some(ctx_ty), Some(start_ty), ReturnType::Type(_, ret_ty)) =
        (tys.first(), tys.get(1), &func.sig.output)
    else {
        return TokenStream::new();
    };
    let ctx_check = quote_spanned!(ctx_ty.span()=> __sortie_ctx(__ctx););
    let start_check = quote_spanned!(start_ty.span()=> __sortie_start(__start););
    let ret_check = quote_spanned!(ret_ty.span()=> __sortie_result(__result););
    quote! {
        #[doc(hidden)]
        #[allow(dead_code, non_snake_case, unused_variables, clippy::all)]
        const _: () = {
            fn __sortie_ctx<M: ::sortie::types::Scenario>(_: &::sortie::Ctx<M>) {}
            fn __sortie_start(_: &::sortie::Start) {}
            fn __sortie_result(_: &::sortie::Result<()>) {}
            fn __sortie_check(__ctx: &#ctx_ty, __start: &#start_ty, __result: &#ret_ty) {
                #ctx_check
                #start_check
                #ret_check
            }
        };
    }
}

/// `fn name(args)`: the span for "missing return type".
fn signature_head(func: &ItemFn) -> TokenStream {
    let sig = &func.sig;
    let mut ts = TokenStream::new();
    sig.asyncness.to_tokens(&mut ts);
    sig.fn_token.to_tokens(&mut ts);
    sig.ident.to_tokens(&mut ts);
    sig.paren_token
        .surround(&mut ts, |inner| sig.inputs.to_tokens(inner));
    ts
}

/// The first tokens of a non-fn item, so the error does not underline a whole struct body.
pub(crate) fn head_of(item: &Item) -> TokenStream {
    let span = match item {
        Item::Struct(s) => s.struct_token.span,
        Item::Enum(e) => e.enum_token.span,
        Item::Impl(i) => i.impl_token.span,
        Item::Mod(m) => m.mod_token.span,
        Item::Const(c) => c.const_token.span,
        Item::Static(s) => s.static_token.span,
        Item::Trait(t) => t.trait_token.span,
        Item::Use(u) => u.use_token.span,
        Item::Type(t) => t.type_token.span,
        _ => return item.to_token_stream(),
    };
    quote_spanned!(span=> item)
}

/// The error plus the original item, so a mistake does not also produce "cannot find function"
/// errors where the mission is used.
pub(crate) fn with_item(err: syn::Error, item: TokenStream) -> TokenStream {
    let mut out = err.into_compile_error();
    out.extend(item);
    out
}

/// Collects several errors so the user sees all of them at once.
#[derive(Default)]
pub(crate) struct Errors(Option<syn::Error>);

impl Errors {
    pub(crate) fn push(&mut self, err: syn::Error) {
        match &mut self.0 {
            Some(all) => all.combine(err),
            None => self.0 = Some(err),
        }
    }

    pub(crate) fn into_error(self) -> Option<syn::Error> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn errors_of(attr: TokenStream, item: TokenStream) -> Vec<String> {
        let out = expand(attr, item).to_string();
        out.split("compile_error")
            .skip(1)
            .map(|s| s.split('"').nth(1).unwrap_or_default().to_owned())
            .collect()
    }

    /// The errors of a mission with these arguments and return type.
    fn errors_of_sig(args: TokenStream, ret: TokenStream) -> Vec<String> {
        errors_of(
            TokenStream::new(),
            quote! { pub async fn run(#args) -> #ret { Ok(()) } },
        )
    }

    /// Passes the syntax checks: emitted with the hidden type checks, which decide the rest.
    fn passes_syntax(args: TokenStream, ret: TokenStream) {
        let out = expand(
            TokenStream::new(),
            quote! { pub async fn run(#args) -> #ret { Ok(()) } },
        )
        .to_string();
        assert!(!out.contains("compile_error"), "{out}");
        assert!(out.contains("__sortie_check"), "{out}");
    }

    #[test]
    fn a_good_mission_is_emitted_with_its_type_checks() {
        let out = expand(
            TokenStream::new(),
            quote! {
                pub async fn run(ctx: Ctx<op_north::OpNorth>, _start: Start) -> sortie::Result<()> {
                    Ok(())
                }
            },
        )
        .to_string();
        assert!(!out.contains("compile_error"), "{out}");
        assert!(out.starts_with("pub async fn run"), "{out}");
        assert!(out.contains("__sortie_check"), "{out}");
        assert!(out.contains(":: sortie :: Ctx < M >"), "{out}");
    }

    #[test]
    fn every_mistake_is_reported_at_once() {
        let errs = errors_of(
            quote!(foo),
            quote! { const unsafe fn run<T>(ctx: &Ctx<T>) where T: Copy {} },
        );
        let joined = errs.join("\n");
        for needle in [
            "takes no arguments",
            "cannot be `const`",
            "cannot be `unsafe`",
            "must be an `async fn`",
            "not generic: name the scenario",
            "remove the `where` clause",
            "by value",
            "exactly two arguments",
            "found 1",
            "declare its return type",
        ] {
            assert!(joined.contains(needle), "missing {needle:?} in\n{joined}");
        }
    }

    #[test]
    fn syntactic_type_mistakes_are_reported_with_the_others() {
        let errs = errors_of(
            TokenStream::new(),
            quote! { fn run(start: Start, ctx: Ctx<Demo>) -> Result<(), String> { Ok(()) } },
        );
        assert_eq!(errs.len(), 3, "{errs:?}");
        assert!(errs[0].contains("must be an `async fn`"), "{errs:?}");
        assert!(errs[1].contains("swapped"), "{errs:?}");
        assert!(errs[2].contains("must return"), "{errs:?}");
    }

    #[test]
    fn swapped_and_misplaced_arguments() {
        let errs = errors_of_sig(quote!(s: Start, c: Ctx<Demo>), quote!(sortie::Result<()>));
        assert_eq!(
            errs,
            ["the arguments are swapped: a mission takes `ctx: Ctx<M>` first, then `start: Start`"]
        );

        let errs = errors_of_sig(quote!(s: sortie::Start, t: u32), quote!(sortie::Result<()>));
        assert_eq!(errs, [format!("`Start` is the second argument: {ORDER}")]);

        let errs = errors_of_sig(
            quote!(a: Ctx<Demo>, b: Ctx<Demo>),
            quote!(sortie::Result<()>),
        );
        assert_eq!(errs, [format!("`Ctx` is the first argument: {ORDER}")]);

        // One argument: the count and the position are both wrong.
        let errs = errors_of_sig(quote!(s: Start), quote!(sortie::Result<()>)).join("\n");
        assert!(errs.contains("`Start` is the second argument"), "{errs}");
        assert!(errs.contains("found 1"), "{errs}");
    }

    #[test]
    fn ctx_takes_one_type_argument() {
        for (ctx, found) in [
            (quote!(Ctx), "Ctx"),
            (quote!(sortie::Ctx<A, B>), "sortie::Ctx<A, B>"),
            (quote!(Ctx<'static>), "Ctx<'static>"),
            (quote!(Ctx<_>), "Ctx<_>"),
            (quote!(Ctx<3>), "Ctx<3>"),
        ] {
            let errs = errors_of_sig(quote!(c: #ctx, s: Start), quote!(sortie::Result<()>));
            assert_eq!(
                errs,
                [format!(
                    "the context takes the scenario marker as its only type argument: \
                     `ctx: Ctx<M>`, e.g. `ctx: Ctx<op_north::OpNorth>`; found `{found}`"
                )],
                "{ctx}"
            );
        }
    }

    #[test]
    fn start_takes_no_generic_arguments() {
        let errs = errors_of_sig(
            quote!(c: Ctx<Demo>, s: Start<u32>),
            quote!(sortie::Result<()>),
        );
        assert_eq!(
            errs,
            ["`Start` takes no generic arguments: `start: Start`; found `Start<u32>`"]
        );
    }

    #[test]
    fn wrong_return_types_show_what_was_written() {
        for (ret, found) in [
            (quote!(Result<(), String>), "Result<(), String>"),
            (
                quote!(std::result::Result<u32, sortie::Error>),
                "std::result::Result<u32, sortie::Error>",
            ),
            (quote!(sortie::Result), "sortie::Result"),
            (
                quote!(sortie::Result<(), Box<dyn Error>>),
                "sortie::Result<(), Box<dyn Error>>",
            ),
            (quote!(Result<(), Error, u8>), "Result<(), Error, u8>"),
            (quote!(()), "()"),
            (quote!((sortie::Result<()>, u8)), "(sortie::Result<()>, u8)"),
            (quote!(!), "!"),
            (
                quote!(&'static sortie::Result<()>),
                "&'static sortie::Result<()>",
            ),
            (quote!([u8]), "[u8]"),
            (quote!([sortie::Result<()>; 2]), "[sortie::Result<()>; 2]"),
            (quote!(*const u8), "*const u8"),
            (quote!(fn(u32) -> bool), "fn(u32) -> bool"),
            (quote!(dyn Fn() + Send), "dyn Fn() + Send"),
            (
                quote!(impl Future<Output = sortie::Result<()>>),
                "impl Future<Output = sortie::Result<()>>",
            ),
            (quote!(_), "_"),
        ] {
            let errs = errors_of_sig(quote!(c: Ctx<Demo>, s: Start), ret.clone());
            assert_eq!(
                errs,
                [format!(
                    "a mission must return `sortie::Result<()>`; found `{found}`"
                )],
                "{ret}"
            );
        }
    }

    #[test]
    fn results_that_may_be_the_sdks_pass_the_syntax_checks() {
        let args = quote!(c: Ctx<Demo>, s: Start);
        for ret in [
            quote!(sortie::Result<()>),
            quote!(Result<()>),
            quote!(::sortie::Result<(), sortie::Error>),
            quote!(Result<(), Error>),
            quote!(core::result::Result<(), SortieError>),
            quote!((sortie::Result<()>)),
        ] {
            passes_syntax(args.clone(), ret);
        }
    }

    #[test]
    fn aliases_and_other_names_still_pass_the_syntax_checks() {
        // `type R = sortie::Result<()>`, `type MyCtx = Ctx<OpNorth>`, `use sortie::Start as S`:
        // only the compiler knows, so the hidden type checks decide.
        passes_syntax(quote!(ctx: MyCtx, start: S), quote!(R));
        passes_syntax(quote!(ctx: Ctx<Demo>, start: Start), quote!(MyResult<()>));
        passes_syntax(quote!(ctx: <X as Y>::Start, start: Start), quote!(u32));
        passes_syntax(
            quote!(ctx: ctx_of!(Demo), start: Start),
            quote!(sortie::Result<()>),
        );
    }

    #[test]
    fn receivers_impl_trait_and_non_functions_are_rejected() {
        let errs = errors_of(
            TokenStream::new(),
            quote! { async fn run(self, s: impl Into<Start>) -> sortie::Result<()> { Ok(()) } },
        )
        .join("\n");
        assert!(errs.contains("free function"), "{errs}");
        assert!(
            errs.contains("`impl Trait` makes the mission generic"),
            "{errs}"
        );

        let errs = errors_of(TokenStream::new(), quote! { struct Run; }).join("\n");
        assert!(errs.contains("goes on the mission's entry point"), "{errs}");
    }

    #[test]
    fn the_original_function_survives_errors() {
        let out = expand(TokenStream::new(), quote! { fn run() {} }).to_string();
        assert!(out.contains("fn run () { }"), "{out}");
    }

    #[test]
    fn written_types_read_as_in_the_source() {
        for (ty, text) in [
            (quote!(Result<(), String>), "Result<(), String>"),
            (quote!(Result<(),>), "Result<(),>"),
            (quote!(&'a mut [u8; 4]), "&'a mut [u8; 4]"),
            (quote!(<T as Trait>::Assoc), "<T as Trait>::Assoc"),
            (quote!(for<'a> fn(&'a u8)), "for<'a> fn(&'a u8)"),
            (quote!(extern "C" fn() -> !), "extern \"C\" fn() -> !"),
        ] {
            assert_eq!(written(&ty), text);
        }
    }
}
