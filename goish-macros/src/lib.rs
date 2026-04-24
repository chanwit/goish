//! goish-macros: internal proc-macro support for the main `goish` crate.
//!
//! Exposes `rewrite_go_body!` — parses a goroutine body, walks the AST, and
//! rewrites goish's sync API calls into their async equivalents:
//!
//!   .Send(x)  →  .send(x).await
//!   .Recv()   →  .recv().await
//!   .Wait()   →  .wait().await
//!
//! This is what lets a `go!{ c.Send(v); let (x, _) = d.Recv(); }` call site
//! read identically to a non-goroutine caller. The rewrite is purely
//! syntactic — there's no type information — so any user method named
//! `Send`/`Recv`/`Wait` inside a `go!{}` body will also be rewritten.
//! Collateral damage is the cost of transparent async.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{
    braced, parenthesized, parse_macro_input, parse_quote, Block, Expr, ExprAwait, Field, Ident,
    Signature, Token, TraitItemFn, Type,
};

/// Walks the AST, rewriting known goish sync calls into async form.
struct GoRewriter;

impl VisitMut for GoRewriter {
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        // Recurse into children first (so e.g. `a.Send(b.Recv())` rewrites
        // the inner `Recv` before the outer `Send`).
        visit_mut::visit_expr_mut(self, node);

        // Rewrite `expr.{Send,Recv,Wait}(...)` → `expr.{send,recv,wait}(...).await`.
        if let Expr::MethodCall(method_call) = node {
            let name = method_call.method.to_string();
            let new_name = match name.as_str() {
                "Send" => Some("send"),
                "Recv" => Some("recv"),
                "Wait" => Some("wait"),
                _ => None,
            };
            if let Some(renamed) = new_name {
                method_call.method = syn::Ident::new(renamed, method_call.method.span());
                // Wrap the whole method call in .await.
                let inner = std::mem::replace(node, parse_quote! { () });
                *node = Expr::Await(ExprAwait {
                    attrs: vec![],
                    base: Box::new(inner),
                    dot_token: Default::default(),
                    await_token: Default::default(),
                });
            }
        }
    }
}

/// `rewrite_go_body!(stmts...)` — used by `goish::go!{}` macro_rules to
/// preprocess the user's body. Accepts raw statements, walks them, emits
/// the rewritten tokens.
#[proc_macro]
pub fn rewrite_go_body(input: TokenStream) -> TokenStream {
    let ts = TokenStream2::from(input);
    // Wrap in braces so syn can parse as a Block.
    let mut block: syn::Block = parse_quote!({ #ts });
    GoRewriter.visit_block_mut(&mut block);
    let stmts = &block.stmts;
    let out = quote! { #(#stmts)* };
    out.into()
}

// ── select! proc macro ────────────────────────────────────────────────
//
// Implements Go's `select` statement with Go-faithful semantics:
//   - arm expressions evaluated exactly once, in source order;
//   - no-default parking via `flume::Selector` (one scheduler cycle
//     wake, no polling);
//   - uniform-random arbitration among ready arms (flume's built-in);
//   - `send(closed_chan, v)` always participates in the pick — will
//     panic even if another recv arm is ready (Go spec compliance).
//
// Emission strategy: per-arm payload `Cell`s carry recv data from the
// Selector's `FnMut` handlers into a post-`wait()` `match __tag.get()`,
// where the user's arm body runs OUTSIDE all closures and therefore
// keeps full `&mut` access to its captured environment. This sidesteps
// the `macro_rules!` attempt's FnMut-borrow conflicts on shared body
// state (e.g. `w.Write` from two arms of an HTTP handler).
//
// See issue #119 for the CSP-derived bug list this macro fixes.

enum Arm {
    RecvNoBind { ch: Expr, body: Block },
    RecvOne    { ch: Expr, v: Ident, body: Block },
    RecvTwo    { ch: Expr, v: Ident, ok: Ident, body: Block },
    Send       { ch: Expr, val: Expr, body: Block },
    Default    { body: Block },
}

struct SelectInput {
    arms: Vec<Arm>,
}

impl Parse for SelectInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut arms = Vec::new();
        while !input.is_empty() {
            arms.push(parse_arm(input)?);
            if input.is_empty() { break; }
            input.parse::<Token![,]>()?;
        }
        Ok(SelectInput { arms })
    }
}

fn parse_arm(input: ParseStream) -> syn::Result<Arm> {
    let kw: Ident = input.parse()?;
    match kw.to_string().as_str() {
        "recv" => parse_recv(input),
        "send" => parse_send(input),
        "default" => {
            input.parse::<Token![=>]>()?;
            let body: Block = input.parse()?;
            Ok(Arm::Default { body })
        }
        _ => Err(syn::Error::new(
            kw.span(),
            format!("select! arms must start with `recv`, `send`, or `default`, got `{}`", kw),
        )),
    }
}

fn parse_recv(input: ParseStream) -> syn::Result<Arm> {
    let content;
    parenthesized!(content in input);
    let ch: Expr = content.parse()?;
    if input.peek(Token![|]) {
        input.parse::<Token![|]>()?;
        let v: Ident = input.parse()?;
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let ok: Ident = input.parse()?;
            input.parse::<Token![|]>()?;
            input.parse::<Token![=>]>()?;
            let body: Block = input.parse()?;
            Ok(Arm::RecvTwo { ch, v, ok, body })
        } else {
            input.parse::<Token![|]>()?;
            input.parse::<Token![=>]>()?;
            let body: Block = input.parse()?;
            Ok(Arm::RecvOne { ch, v, body })
        }
    } else {
        input.parse::<Token![=>]>()?;
        let body: Block = input.parse()?;
        Ok(Arm::RecvNoBind { ch, body })
    }
}

fn parse_send(input: ParseStream) -> syn::Result<Arm> {
    let content;
    parenthesized!(content in input);
    let ch: Expr = content.parse()?;
    content.parse::<Token![,]>()?;
    let val: Expr = content.parse()?;
    input.parse::<Token![=>]>()?;
    let body: Block = input.parse()?;
    Ok(Arm::Send { ch, val, body })
}

/// `select!` — Go's select statement with Go-faithful semantics.
///
/// # Syntax
///
/// ```ignore
/// select! {
///     recv(ch) => { body },            // receive, discard value
///     recv(ch) |v| => { body },        // bind value
///     recv(ch) |v, ok| => { body },    // bind value and open flag
///     send(ch, expr) => { body },      // send expr, run body on success
///     default => { body },             // non-blocking fallback
/// }
/// ```
///
/// # Early return pattern (gotcha)
///
/// `return` inside an arm body WORKS at runtime — control leaves the
/// enclosing function — but Rust's type checker can't see that. The
/// macro expands to sequential `if` statements, each without `else`:
///
/// ```ignore
/// if !__fired { if ready { body; __fired = true; } }
/// if !__fired { default_body }
/// ```
///
/// Rust infers the `if`-without-`else` as `()` even when `body` ends in
/// `return`, so a function like
///
/// ```ignore
/// fn try_send(ch: &Chan<()>) -> bool {
///     select! {
///         send(ch, ()) => { return true; },
///         default => { return false; },
///     }
///     // error: expected bool, found ()
/// }
/// ```
///
/// fails to compile. Rust won't trace `return` through the expansion
/// back to the function boundary.
///
/// **Idiomatic Rust workaround** — use an outer mutable, then return:
///
/// ```ignore
/// fn try_send(ch: &Chan<()>) -> bool {
///     let mut taken = false;
///     select! {
///         send(ch, ()) => { taken = true; },
///         default => {},
///     }
///     taken
/// }
/// ```
///
/// This is a fundamental Rust/Go difference: Go's `case` bodies are
/// statement sequences that can `return` directly; Rust needs a unified
/// expression type across all arms, and the macro can't distinguish
/// at expansion time whether a body diverges or flows through.
#[proc_macro]
pub fn select(input: TokenStream) -> TokenStream {
    let SelectInput { arms } = parse_macro_input!(input as SelectInput);
    let mut channel_arms = Vec::new();
    let mut default_body: Option<Block> = None;
    for arm in arms {
        match arm {
            Arm::Default { body } => {
                if default_body.is_some() {
                    return syn::Error::new_spanned(
                        &body, "select! may have at most one `default` arm",
                    ).to_compile_error().into();
                }
                default_body = Some(body);
            }
            other => channel_arms.push(other),
        }
    }
    if let Some(def) = default_body {
        emit_with_default(&channel_arms, &def).into()
    } else {
        emit_no_default(&channel_arms).into()
    }
}

fn emit_with_default(arms: &[Arm], default_body: &Block) -> TokenStream2 {
    // Phase 1: evaluate every arm's channel + send-value expression once,
    // in source order (Go spec). Phase 2: non-blocking try each arm; if
    // none fires, run default.
    let mut setup = TokenStream2::new();
    let mut tries = TokenStream2::new();
    for (i, arm) in arms.iter().enumerate() {
        let ch = format_ident!("__arm_{}_ch", i);
        let val = format_ident!("__arm_{}_val", i);
        match arm {
            Arm::RecvNoBind { ch: e, body } => {
                setup.extend(quote! { let #ch = &(#e); });
                tries.extend(quote! {
                    if !__fired {
                        if #ch.__select_try_recv().is_some() {
                            #body
                            __fired = true;
                        }
                    }
                });
            }
            Arm::RecvOne { ch: e, v, body } => {
                setup.extend(quote! { let #ch = &(#e); });
                tries.extend(quote! {
                    if !__fired {
                        if let ::std::option::Option::Some((#v, _)) = #ch.__select_try_recv() {
                            #body
                            __fired = true;
                        }
                    }
                });
            }
            Arm::RecvTwo { ch: e, v, ok, body } => {
                setup.extend(quote! { let #ch = &(#e); });
                tries.extend(quote! {
                    if !__fired {
                        if let ::std::option::Option::Some((#v, #ok)) = #ch.__select_try_recv() {
                            #body
                            __fired = true;
                        }
                    }
                });
            }
            Arm::Send { ch: e, val: vexpr, body } => {
                setup.extend(quote! {
                    let #ch = &(#e);
                    let mut #val = ::std::option::Option::Some(#vexpr);
                });
                tries.extend(quote! {
                    if !__fired {
                        if let ::std::option::Option::Some(__v) = #val.take() {
                            match #ch.__select_try_send(__v) {
                                ::std::result::Result::Ok(()) => {
                                    #body
                                    __fired = true;
                                }
                                ::std::result::Result::Err(__returned) => {
                                    // Buffer full; drop the returned value.
                                    // (Go spec: send evaluated, not committed.)
                                    let _ = __returned;
                                }
                            }
                        }
                    }
                });
            }
            Arm::Default { .. } => unreachable!(),
        }
    }
    quote! {{
        #setup
        #[allow(unused_mut, unused_assignments)]
        let mut __fired = false;
        #tries
        if !__fired {
            #default_body
        }
    }}
}

fn emit_no_default(arms: &[Arm]) -> TokenStream2 {
    // Phase 1: evaluate expressions once. Phase 2: build flume::Selector
    // with two arms per input (main chan + shadow close_rx). Phase 3:
    // dispatch by tag AFTER .wait() so user bodies run outside FnMut
    // closures.
    let mut setup = TokenStream2::new();
    let mut chain = TokenStream2::new();
    let mut dispatch = TokenStream2::new();
    for (i, arm) in arms.iter().enumerate() {
        let ch = format_ident!("__arm_{}_ch", i);
        let val = format_ident!("__arm_{}_val", i);
        let payload = format_ident!("__arm_{}_payload", i);
        let tag = i as u32;
        match arm {
            Arm::RecvNoBind { ch: e, body } => {
                setup.extend(quote! { let #ch = &(#e); });
                chain.extend(quote! {
                    .recv(#ch.__flume_rx(), |__res| {
                        __tag.set(#tag);
                        // consume and drop the value
                        let _ = __res;
                    })
                    .recv(#ch.__flume_close_rx(), |_: ::std::result::Result<(), ::goish::__flume::RecvError>| {
                        __tag.set(#tag);
                        let _ = #ch.__flume_rx().try_recv();
                    })
                });
                dispatch.extend(quote! {
                    #tag => #body,
                });
            }
            Arm::RecvOne { ch: e, v, body } => {
                setup.extend(quote! {
                    let #ch = &(#e);
                    let #payload: ::std::cell::Cell<::std::option::Option<_>> =
                        ::std::cell::Cell::new(::std::option::Option::None);
                });
                chain.extend(quote! {
                    .recv(#ch.__flume_rx(), |__res| {
                        __tag.set(#tag);
                        let __p: (_, bool) = match __res {
                            ::std::result::Result::Ok(__x) => (__x, true),
                            ::std::result::Result::Err(_) => (::std::default::Default::default(), false),
                        };
                        #payload.set(::std::option::Option::Some(__p));
                    })
                    .recv(#ch.__flume_close_rx(), |_: ::std::result::Result<(), ::goish::__flume::RecvError>| {
                        __tag.set(#tag);
                        let __p: (_, bool) = match #ch.__flume_rx().try_recv() {
                            ::std::result::Result::Ok(__x) => (__x, true),
                            ::std::result::Result::Err(_) => (::std::default::Default::default(), false),
                        };
                        #payload.set(::std::option::Option::Some(__p));
                    })
                });
                dispatch.extend(quote! {
                    #tag => {
                        let (#v, _) = #payload.take().expect("select! arm fired but payload missing");
                        #body
                    },
                });
            }
            Arm::RecvTwo { ch: e, v, ok, body } => {
                setup.extend(quote! {
                    let #ch = &(#e);
                    let #payload: ::std::cell::Cell<::std::option::Option<_>> =
                        ::std::cell::Cell::new(::std::option::Option::None);
                });
                chain.extend(quote! {
                    .recv(#ch.__flume_rx(), |__res| {
                        __tag.set(#tag);
                        let __p: (_, bool) = match __res {
                            ::std::result::Result::Ok(__x) => (__x, true),
                            ::std::result::Result::Err(_) => (::std::default::Default::default(), false),
                        };
                        #payload.set(::std::option::Option::Some(__p));
                    })
                    .recv(#ch.__flume_close_rx(), |_: ::std::result::Result<(), ::goish::__flume::RecvError>| {
                        __tag.set(#tag);
                        let __p: (_, bool) = match #ch.__flume_rx().try_recv() {
                            ::std::result::Result::Ok(__x) => (__x, true),
                            ::std::result::Result::Err(_) => (::std::default::Default::default(), false),
                        };
                        #payload.set(::std::option::Option::Some(__p));
                    })
                });
                dispatch.extend(quote! {
                    #tag => {
                        let (#v, #ok) = #payload.take().expect("select! arm fired but payload missing");
                        #body
                    },
                });
            }
            Arm::Send { ch: e, val: vexpr, body } => {
                setup.extend(quote! {
                    let #ch = &(#e);
                    // Go spec: send on closed channel is "always ready" and
                    // panics. Pre-check before entering Selector so the
                    // panic fires even if another arm would also be ready.
                    if #ch.__is_closed() {
                        ::std::panic!("send on closed channel");
                    }
                    let #val = #vexpr;
                });
                chain.extend(quote! {
                    .send(#ch.__flume_tx(), #val, |__res| {
                        __tag.set(#tag);
                        // Re-check: channel may have closed between Selector
                        // build and handler fire. At the flume level the
                        // main tx is still alive (we only drop the shadow
                        // sender on Close), so flume's send may report Ok
                        // even though our close flag is set.
                        if #ch.__is_closed() || __res.is_err() {
                            ::std::panic!("send on closed channel");
                        }
                    })
                    .recv(#ch.__flume_close_rx(), |_: ::std::result::Result<(), ::goish::__flume::RecvError>| {
                        // Shadow close_rx disconnected → channel closed
                        // during Selector wait. Panic per Go spec.
                        ::std::panic!("send on closed channel");
                    })
                });
                dispatch.extend(quote! {
                    #tag => #body,
                });
            }
            Arm::Default { .. } => unreachable!(),
        }
    }
    quote! {{
        #setup
        let __tag: ::std::cell::Cell<u32> = ::std::cell::Cell::new(::std::u32::MAX);
        ::goish::__flume::Selector::new()
            #chain
            .wait();
        match __tag.get() {
            #dispatch
            _ => ::std::unreachable!("select! wait() returned without firing any arm"),
        }
    }}
}

// ── Interface! proc-macro ─────────────────────────────────────────────
//
// Go: type Core interface { With(fields []Field) Core; Write(msg string) }
//
// Goish (decl form):
//
//   Interface!{
//       type Core interface {
//           fn Write(&self, msg: &str);
//           fn With(&self, tag: &'static str) -> Core;
//       }
//   }
//
// Emits:
//   - `#[doc(hidden)] pub trait __CoreTrait: DynClone + Send + Sync { ... }`
//   - `clone_trait_object!(__CoreTrait)`
//   - `pub struct Core(Box<dyn __CoreTrait>)` with `impl Clone`
//   - forwarding inherent methods on Core
//   - `impl<T: __CoreTrait + 'static> From<T> for Core`
//
// Goish (impl form):
//
//   Interface!{
//       impl Core for InMem {
//           fn Write(&self, msg: &str) { ... }
//           fn With(&self, tag: &'static str) -> Core { /* ... */ }
//       }
//   }
//
// Emits:  `impl __CoreTrait for InMem { <user methods> }`
//
// The hidden trait name is `__<Name>Trait`. Users never name it directly —
// they always go through `Interface!{ impl Name for Type { ... } }`.

enum InterfaceInput {
    Decl(InterfaceDecl),
    Impl(InterfaceImpl),
}

struct InterfaceDecl {
    name: Ident,
    supers: Vec<syn::TypeParamBound>,
    methods: Vec<TraitItemFn>,
}

struct InterfaceImpl {
    iface: syn::Path,
    target: Type,
    methods: Vec<syn::ImplItemFn>,
}

impl Parse for InterfaceInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![type]) {
            input.parse::<Token![type]>()?;
            let name: Ident = input.parse()?;

            // Optional supertrait clause: `: Super1 + Super2 + …`
            //   - bare ident  → mangled to `__<Ident>Trait`
            //   - path/other  → verbatim (existing trait like `io::Writer`)
            let mut supers: Vec<syn::TypeParamBound> = Vec::new();
            if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                loop {
                    let bound = parse_super_bound(input)?;
                    supers.push(bound);
                    if input.peek(Token![+]) {
                        input.parse::<Token![+]>()?;
                    } else {
                        break;
                    }
                }
            }

            let kw: Ident = input.parse()?;
            if kw != "interface" {
                return Err(syn::Error::new(
                    kw.span(),
                    "Interface!: expected `interface` keyword",
                ));
            }
            let body;
            braced!(body in input);
            let mut methods = Vec::new();
            while !body.is_empty() {
                methods.push(body.parse::<TraitItemFn>()?);
            }
            Ok(InterfaceInput::Decl(InterfaceDecl { name, supers, methods }))
        } else if lookahead.peek(Token![impl]) {
            input.parse::<Token![impl]>()?;
            // Accept a full path (`mymod::Core`) or bare ident (`Core`).
            // The trait name is derived by mangling only the last segment,
            // preserving module qualification so cross-module impls don't
            // leak `__XTrait` into user scope (friction #58).
            let iface: syn::Path = input.parse()?;
            input.parse::<Token![for]>()?;
            let target: Type = input.parse()?;
            let body;
            braced!(body in input);
            let mut methods = Vec::new();
            while !body.is_empty() {
                methods.push(body.parse::<syn::ImplItemFn>()?);
            }
            Ok(InterfaceInput::Impl(InterfaceImpl { iface, target, methods }))
        } else {
            Err(lookahead.error())
        }
    }
}

/// Parse one supertrait in the `: A + B + …` clause. A bare `Ident` is
/// rewritten to `__<Ident>Trait` (Interface!-declared). Any other path
/// (`io::Writer`, `some::Trait<T>`) is kept verbatim.
fn parse_super_bound(input: ParseStream) -> syn::Result<syn::TypeParamBound> {
    // `syn::Path` greedily consumes a single ident OR a full path.
    let path: syn::Path = input.parse()?;
    let is_bare_ident = path.segments.len() == 1
        && path.leading_colon.is_none()
        && path.segments[0].arguments.is_none();
    let resolved_path = if is_bare_ident {
        let id = &path.segments[0].ident;
        let mangled = format_ident!("__{}Trait", id);
        syn::parse_quote!(#mangled)
    } else {
        path
    };
    Ok(syn::TypeParamBound::Trait(syn::TraitBound {
        paren_token: None,
        modifier: syn::TraitBoundModifier::None,
        lifetimes: None,
        path: resolved_path,
    }))
}

/// `Interface!` — Go's `type X interface { ... }` plus `impl X for T`.
///
/// See `REFERENCES.md` §11 for Go → Goish side-by-side examples.
#[proc_macro]
#[allow(non_snake_case)]
pub fn Interface(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as InterfaceInput);
    match parsed {
        InterfaceInput::Decl(d) => interface_decl_emit(d).into(),
        InterfaceInput::Impl(i) => interface_impl_emit(i).into(),
    }
}

fn interface_decl_emit(d: InterfaceDecl) -> TokenStream2 {
    let name = &d.name;
    let trait_name = format_ident!("__{}Trait", name);

    let mut trait_methods = TokenStream2::new();
    let mut forwards = TokenStream2::new();
    for m in &d.methods {
        let sig = &m.sig;
        trait_methods.extend(quote! { #sig ; });
        forwards.extend(emit_forward(sig));
    }

    let supers = &d.supers;

    quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types, non_snake_case)]
        pub trait #trait_name:
            #( #supers + )*
            ::goish::__DynClone + ::std::marker::Send + ::std::marker::Sync
        {
            #trait_methods
        }
        ::goish::clone_trait_object!(#trait_name);

        #[allow(non_camel_case_types, non_snake_case)]
        pub struct #name(
            #[doc(hidden)]
            pub ::std::boxed::Box<dyn #trait_name>,
        );

        impl ::std::clone::Clone for #name {
            fn clone(&self) -> Self { #name(::std::clone::Clone::clone(&self.0)) }
        }

        #[allow(non_snake_case)]
        impl #name {
            #forwards
        }

        impl<__GoishT> ::std::convert::From<__GoishT> for #name
        where __GoishT: #trait_name + 'static
        {
            fn from(t: __GoishT) -> Self { #name(::std::boxed::Box::new(t)) }
        }
    }
}

/// Emit `pub fn Name(&self, ...) -> Ret { self.0.Name(...) }` from a trait
/// method signature. Each non-receiver param is forwarded by its ident.
fn emit_forward(sig: &Signature) -> TokenStream2 {
    let name = &sig.ident;
    // Extract non-receiver parameter identifiers for the call site.
    let mut call_args = TokenStream2::new();
    let mut first = true;
    for input in sig.inputs.iter() {
        if let syn::FnArg::Typed(pt) = input {
            if !first { call_args.extend(quote! { , }); }
            first = false;
            match &*pt.pat {
                syn::Pat::Ident(pi) => {
                    let id = &pi.ident;
                    call_args.extend(quote! { #id });
                }
                other => {
                    // Fallback: emit as-is (rare — macro users should pass simple idents).
                    call_args.extend(quote! { #other });
                }
            }
        }
    }
    quote! {
        pub #sig { self.0.#name(#call_args) }
    }
}

fn interface_impl_emit(i: InterfaceImpl) -> TokenStream2 {
    let mut trait_path = i.iface.clone();
    if let Some(last) = trait_path.segments.last_mut() {
        last.ident = format_ident!("__{}Trait", last.ident);
    }
    let target = &i.target;
    let methods = &i.methods;
    quote! {
        impl #trait_path for #target {
            #( #methods )*
        }
    }
}

// ── ErrorType! proc-macro ─────────────────────────────────────────────
//
// Go: type MultiError struct { errs []error }
//     func (m *MultiError) Error() string { /* ... */ }
//
// Goish:
//
//   ErrorType!{
//       type MultiError struct {
//           errs: slice<error>,
//       }
//       fn Error(&self) -> string {
//           /* user body */
//       }
//   }
//
// Emits:
//   - `#[derive(Clone, Debug)] pub struct MultiError { pub errs: slice<error> }`
//   - `impl MultiError { pub fn Error(&self) -> string { <user body> } }`
//   - `impl Display for MultiError { fmt via self.Error() }`
//   - `impl GoishError for MultiError { as_any -> self }`
//   - `impl From<MultiError> for error { via errors::FromDyn }`
//
// User code then writes `return MultiError { ... }.into();` — no `FromDyn`
// visible, no `as_any` visible, no Rust trait-object juggling.

struct ErrorTypeInput {
    name: Ident,
    fields: Punctuated<Field, Token![,]>,
    error_body: Block,
    error_ret: Type,
}

impl Parse for ErrorTypeInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![type]>()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![struct]>()?;
        let body;
        braced!(body in input);
        let fields = body.parse_terminated(Field::parse_named, Token![,])?;

        // Now parse the `fn Error(&self) -> TYPE { body }`
        input.parse::<Token![fn]>()?;
        let err_ident: Ident = input.parse()?;
        if err_ident != "Error" {
            return Err(syn::Error::new(
                err_ident.span(),
                "ErrorType!: expected `fn Error(&self) -> <type> { ... }`",
            ));
        }
        let args;
        parenthesized!(args in input);
        args.parse::<Token![&]>()?;
        args.parse::<Token![self]>()?;
        if !args.is_empty() {
            return Err(syn::Error::new(
                args.span(),
                "ErrorType!: `fn Error` takes only `&self`",
            ));
        }
        input.parse::<Token![->]>()?;
        let error_ret: Type = input.parse()?;
        let error_body: Block = input.parse()?;

        Ok(ErrorTypeInput { name, fields, error_body, error_ret })
    }
}

/// `ErrorType!` — declare a user error type with `Error() string`.
///
/// See `REFERENCES.md` §12 for Go → Goish side-by-side examples.
#[proc_macro]
#[allow(non_snake_case)]
pub fn ErrorType(input: TokenStream) -> TokenStream {
    let ErrorTypeInput { name, fields, error_body, error_ret } =
        parse_macro_input!(input as ErrorTypeInput);

    let field_iter = fields.iter();
    quote! {
        #[derive(::std::clone::Clone, ::std::fmt::Debug)]
        pub struct #name {
            #( pub #field_iter, )*
        }

        #[allow(non_snake_case)]
        impl #name {
            pub fn Error(&self) -> #error_ret #error_body
        }

        impl ::std::fmt::Display for #name {
            fn fmt(&self, __f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::write!(__f, "{}", #name::Error(self))
            }
        }

        impl ::goish::errors::GoishError for #name {
            fn as_any(&self) -> &dyn ::std::any::Any { self }
        }

        impl ::std::convert::From<#name> for ::goish::errors::error {
            fn from(__e: #name) -> ::goish::errors::error {
                ::goish::errors::__FromDyn(__e)
            }
        }
    }
    .into()
}
