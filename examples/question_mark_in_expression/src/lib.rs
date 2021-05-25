#![feature(rustc_private)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_session;

mod question_mark_in_expression;

#[no_mangle]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[question_mark_in_expression::QUESTION_MARK_IN_EXPRESSION]);
    lint_store
        .register_late_pass(|| Box::new(question_mark_in_expression::QuestionMarkInExpression));
}

#[test]
fn ui_example() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "clone");
}

#[test]
fn ui_examples() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
