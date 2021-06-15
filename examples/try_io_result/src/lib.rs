#![feature(rustc_private)]
#![recursion_limit = "256"]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;

mod try_io_result;

#[no_mangle]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[try_io_result::TRY_IO_RESULT]);
    lint_store.register_late_pass(|| Box::new(try_io_result::TryIoResult));
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
