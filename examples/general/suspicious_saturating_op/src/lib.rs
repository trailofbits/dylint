#![feature(rustc_private)]
#![recursion_limit = "256"]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_span::symbol::Ident;

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Checks for consecutive saturating operations.
    ///
    /// ### Why is this bad?
    /// If the first operation saturates, the second operation may produce an incorrect result.
    ///
    /// ### Example
    /// ```rust
    /// # fn foo() -> Option<u64> {
    /// # let mut x: u64 = 1;
    /// # let y: u64 = 1;
    /// # let z: u64 = 1;
    /// x = x.saturating_add(y).saturating_sub(z);
    /// # None
    /// # }
    /// ```
    /// Use instead:
    /// ```rust
    /// # fn foo() -> Option<i32> {
    /// # let mut x: u64 = 1;
    /// # let y: u64 = 1;
    /// # let z: u64 = 1;
    /// x = x.checked_add(y)?;
    /// x = x.checked_sub(z)?;
    /// # None
    /// # }
    /// ```
    pub SUSPICIOUS_SATURATING_OP,
    Warn,
    "consecutive saturating operations"
}

const SIGNED_OR_UNSIGNED: &[[&'static str; 2]] = &[
    ["div", "mul"],
    ["div", "pow"],
    ["mul", "div"],
    ["pow", "div"],
];

const UNSIGNED_ONLY: &[[&'static str; 2]] = &[
    ["add", "div"],
    ["add", "sub"],
    ["sub", "add"],
    ["sub", "mul"],
    ["sub", "pow"],
];

impl<'tcx> LateLintPass<'tcx> for SuspiciousSaturatingOp {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'_>) {
        if_chain! {
            if let ExprKind::MethodCall(second, receiver_1, _, _) = expr.kind;
            if second.args.is_none();
            if let ExprKind::MethodCall(first, receiver_0, _, _) = receiver_1.kind;
            if first.args.is_none();
            then {
                for pattern in SIGNED_OR_UNSIGNED {
                    if check(cx, expr, &first.ident, &second.ident, pattern) {
                        return;
                    }
                }
                let ty = cx.typeck_results().expr_ty(receiver_0);
                if matches!(ty.kind(), ty::Uint(_)) {
                    for pattern in UNSIGNED_ONLY {
                        if check(cx, expr, &first.ident, &second.ident, pattern) {
                            return;
                        }
                    }
                }
            }
        }
    }
}

fn check(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    first: &Ident,
    second: &Ident,
    pattern: &[&'static str; 2],
) -> bool {
    if first.as_str() == String::from("saturating_") + pattern[0]
        && (second.as_str() == pattern[1]
            || second.as_str().ends_with(&(String::from("_") + pattern[1])))
    {
        span_lint(
            cx,
            SUSPICIOUS_SATURATING_OP,
            expr.span,
            &format!("suspicious use of `{}` followed by `{}`", first, second),
        );
        true
    } else {
        false
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}
