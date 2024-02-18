#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::LitKind;
use rustc_hir::{Expr, ExprKind, ItemKind, Node, OwnerNode};
use rustc_lint::{LateContext, LateLintPass};
use serde::Deserialize;

dylint_linting::impl_late_lint! {
    /// ### What it does
    /// Checks for unnamed constants, aka magic numbers.
    ///
    /// ### Why is this bad?
    /// "Magic numbers are considered bad practice in programming, because they can make the code
    /// more difficult to understand and harder to maintain." ([pandaquests])
    ///
    /// ### Example
    /// ```rust
    /// # let mut x: u64 = 1;
    /// x *= 1000;
    /// ```
    /// Use instead:
    /// ```rust
    /// # let mut x: u64 = 1;
    /// const MILLIS: u64 = 1000;
    /// x *= MILLIS;
    /// ```
    ///
    /// ### Configuration
    /// - `threshold: u64` (default `1000`): Minimum value a constant must exceed to be flagged.
    ///
    /// [pandaquests]: https://levelup.gitconnected.com/whats-so-bad-about-magic-numbers-4c0a0c524b7d
    pub UNNAMED_CONSTANT,
    Warn,
    "unnamed constants, aka magic numbers",
    UnnamedConstant::new()
}

#[derive(Deserialize)]
struct Config {
    threshold: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self { threshold: 1000 }
    }
}

struct UnnamedConstant {
    config: Config,
}

impl UnnamedConstant {
    pub fn new() -> Self {
        Self {
            config: dylint_linting::config_or_default(env!("CARGO_PKG_NAME")),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for UnnamedConstant {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if !cx
                .tcx
                .hir()
                .parent_iter(expr.hir_id)
                .any(|(hir_id, _)| cx.tcx.hir().span(hir_id).from_expansion())

            // smoelius: Only flag expressions that appear within other expressions (as opposed to,
            // e.g., array bounds).
            && matches!(cx.tcx.hir().get_parent(expr.hir_id), Node::Expr(_))

            // smoelius: And those other expressions must not appear within a constant declaration.
            && let owner_id = cx.tcx.hir().get_parent_item(expr.hir_id)
            && let OwnerNode::Item(item) = cx.tcx.hir_owner_node(owner_id)
            && !matches!(item.kind, ItemKind::Const(..))

            && let ExprKind::Lit(lit) = expr.kind
            && let LitKind::Int(value, _) = lit.node
            && value >= u128::from(self.config.threshold)
        {
            span_lint_and_help(
                cx,
                UNNAMED_CONSTANT,
                expr.span,
                "unnamed constant",
                None,
                "give the constant a name and use that instead",
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}

#[test]
fn ui_threshold() {
    dylint_testing::ui::Test::src_base(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui_threshold"),
    )
    .dylint_toml("unnamed_constant.threshold = 2")
    .run();
}

#[test]
fn ui_main_rs_equal() {
    let ui_main_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/main.rs"),
    )
    .unwrap();
    let ui_threshold_main_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui_threshold/main.rs"),
    )
    .unwrap();
    assert_eq!(ui_main_rs, ui_threshold_main_rs);
}
