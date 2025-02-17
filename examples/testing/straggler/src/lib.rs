#![feature(rustc_private)]
#![warn(unused_extern_crates)]

use rustc_lint::LateLintPass;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// This lint does nothing. Its Rust toolchain is intentionally held back for testing purposes.
    ///
    /// ### Why is this bad?
    ///
    /// It's not.
    ///
    /// ### Known problems
    ///
    /// This lint does nothing.
    pub STRAGGLER,
    Allow,
    "this lint does nothing"
}

impl<'tcx> LateLintPass<'tcx> for Straggler {}
