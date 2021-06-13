#![feature(rustc_private)]
#![recursion_limit = "256"]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_session;

mod path_separator_in_string_literal;

#[no_mangle]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store
        .register_lints(&[path_separator_in_string_literal::PATH_SEPARATOR_IN_STRING_LITERAL]);
    lint_store.register_late_pass(|| {
        Box::new(path_separator_in_string_literal::PathSeparatorInStringLiteral)
    });
}

#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}
