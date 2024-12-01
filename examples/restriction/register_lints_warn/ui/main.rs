#![feature(rustc_private)]

extern crate rustc_lint;
extern crate rustc_session;

pub fn register_lints(sess: &rustc_session::Session, _lint_store: &mut rustc_lint::LintStore) {
    sess.dcx().warn("something bad happened");
}

use rustc_lint::LintContext;

struct LintPass;

impl rustc_lint::LintPass for LintPass {
    fn name(&self) -> &'static str {
        "lint_pass"
    }
    fn get_lints(&self) -> Vec<&'static rustc_lint::Lint> {
        Vec::new()
    }
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for LintPass {
    fn check_crate(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
        cx.sess().dcx().warn("something bad happened");
    }
}

fn main() {}
