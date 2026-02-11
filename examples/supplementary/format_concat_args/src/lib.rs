#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

use clippy_utils::{
    diagnostics::span_lint_and_sugg,
    is_expn_of,
};
use rustc_hir::Expr;
use rustc_lint::{Lint, LateContext, LateLintPass, Level};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

// Declare the lint directly
pub static FORMAT_CONCAT_ARGS: &Lint = &Lint {
    name: "format_concat_args",
    default_level: Level::Allow,
    desc: "Checks for `format!(...)` invocations where `concat!(...)` could be used instead.",
    edition_lint_opts: None,
    report_in_external_macro: true,
    future_incompatible: None,
    is_externally_loaded: false,
    eval_always: false,
    feature_gate: None,
    crate_level_only: false,
};

// Declare the lint pass
declare_lint_pass!(FormatConcatArgs => [FORMAT_CONCAT_ARGS]);

impl<'tcx> LateLintPass<'tcx> for FormatConcatArgs {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        // Check if the expression is a `format!` macro invocation
        if is_expn_of(expr.span, sym::format_args.as_str()).is_some() {
            let expn_data = expr.span.ctxt().outer_expn_data();

            // Ensure this is from `std::format!` specifically
            if expn_data.macro_def_id != cx.tcx.get_diagnostic_item(sym::format_macro) {
                return; // Not a std::format! macro call
            }

            span_lint_and_sugg(
                cx,
                FORMAT_CONCAT_ARGS,
                expr.span,
                "this `format!(...)` invocation might be replaceable with `concat!(...)`",
                "consider using concat! if all arguments are constant",
                "concat!(...)".to_string(),
                rustc_errors::Applicability::HasPlaceholders,
            );
        }
    }
}

#[cfg(not(feature = "rlib"))]
#[allow(unused_extern_crates)]
#[allow(clippy::float_arithmetic, clippy::option_option, clippy::unreachable)]
fn main() {
   dylint_linting::test(env!("CARGO_PKG_NAME"), &[]);
}