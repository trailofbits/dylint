#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_span;

use clippy_utils::{diagnostics::span_lint, sym};
use rustc_ast::{
    token::{LitKind, TokenKind},
    tokenstream::TokenTree,
    Expr, ExprKind, Item, NodeId,
};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_span::sym;

dylint_linting::impl_pre_expansion_lint! {
    /// ### What it does
    /// Checks for `env!` or `option_env!` applied outside of a test to a Cargo environment variable
    /// containing a path, e.g., `CARGO_MANIFEST_DIR`.
    ///
    /// ### Why is this bad?
    /// The path might not exist when the code is used in production.
    ///
    /// ### Known problems
    /// The lint does not apply inside macro arguments. So false negatives could result.
    ///
    /// ### Example
    /// ```rust
    /// fn main() {
    ///     let path = option_env!("CARGO");
    ///     println!("{:?}", path);
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// fn main() {
    ///     let path = std::env::var("CARGO");
    ///     println!("{:?}", path);
    /// }
    /// ```
    pub ENV_CARGO_PATH,
    Warn,
    "`env!` applied to Cargo environment variables containing paths",
    EnvCargoPath::default()
}

#[derive(Default)]
pub struct EnvCargoPath {
    stack: Vec<NodeId>,
}

impl EarlyLintPass for EnvCargoPath {
    fn check_item(&mut self, _cx: &EarlyContext, item: &Item) {
        if self.in_test_item() || is_test_item(item) {
            self.stack.push(item.id);
        }
    }

    fn check_item_post(&mut self, _cx: &EarlyContext, item: &Item) {
        if let Some(node_id) = self.stack.pop() {
            assert_eq!(node_id, item.id);
        }
    }

    fn check_expr(&mut self, cx: &EarlyContext, expr: &Expr) {
        if !self.in_test_item()
            && let ExprKind::MacCall(mac) = &expr.kind
            && (mac.path == sym!(env) || mac.path == sym!(option_env))
            && let [TokenTree::Token(token, _)] =
                mac.args.tokens.trees().collect::<Vec<_>>().as_slice()
            && let TokenKind::Literal(lit) = token.kind
            && lit.kind == LitKind::Str
            && is_blacklisted_variable(lit.symbol.as_str())
        {
            span_lint(
                cx,
                ENV_CARGO_PATH,
                expr.span,
                "this path might not exist in production",
            );
        }
    }
}

impl EnvCargoPath {
    fn in_test_item(&self) -> bool {
        !self.stack.is_empty()
    }
}

fn is_test_item(item: &Item) -> bool {
    item.attrs.iter().any(|attr| {
        if attr.has_name(sym::test) {
            true
        } else {
            if attr.has_name(sym::cfg)
                && let Some(items) = attr.meta_item_list()
                && let [item] = items.as_slice()
                && let Some(feature_item) = item.meta_item()
                && feature_item.has_name(sym::test)
            {
                true
            } else {
                false
            }
        }
    })
}

fn is_blacklisted_variable(var: &str) -> bool {
    var == "CARGO" || var == "CARGO_MANIFEST_DIR" || var.starts_with("CARGO_BIN_EXE_")
}

#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}
