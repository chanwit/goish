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
use quote::quote;
use syn::visit_mut::{self, VisitMut};
use syn::{parse_quote, Expr, ExprAwait};

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
