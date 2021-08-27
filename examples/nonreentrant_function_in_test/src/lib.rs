#![feature(rustc_private)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_ast;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

mod nonreentrant_function_in_test;

#[no_mangle]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[nonreentrant_function_in_test::NONREENTRANT_FUNCTION_IN_TEST]);
    lint_store.register_pre_expansion_pass(|| {
        Box::new(nonreentrant_function_in_test::NonreentrantFunctionInTest::default())
    });
}

#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}
