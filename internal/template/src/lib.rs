#![feature(rustc_private)]
// #![warn(unused_extern_crates)]

// A list of available compiler crates can be found here:
// https://doc.rust-lang.org/nightly/nightly-rustc/
extern crate rustc_arena;
extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_hir_pretty;
extern crate rustc_index;
extern crate rustc_infer;
extern crate rustc_lexer;
extern crate rustc_middle;
extern crate rustc_mir_dataflow;
extern crate rustc_parse;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_trait_selection;

use rustc_lint::LateLintPass;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Known problems
    ///
    /// Remove if none.
    ///
    /// ### Example
    ///
    /// ```rust
    /// // example code where a warning is issued
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// // example code that does not raise a warning
    /// ```
    pub FILL_ME_IN,
    Warn,
    "description goes here"
}

impl<'tcx> LateLintPass<'tcx> for FillMeIn {
    // A list of things you might check can be found here:
    // https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
