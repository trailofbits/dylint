#![feature(rustc_private)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_lint;
extern crate rustc_session;

#[expect(clippy::no_mangle_with_rust_abi)]
#[unsafe(no_mangle)]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // smoelius: Please keep the following `register_lints` calls sorted by crate name.
    abs_home_path::register_lints(sess, lint_store);
    await_holding_span_guard::register_lints(sess, lint_store);
    basic_dead_store::register_lints(sess, lint_store);
    crate_wide_allow::register_lints(sess, lint_store);
    incorrect_matches_operation::register_lints(sess, lint_store);
    non_local_effect_before_error_return::register_lints(sess, lint_store);
    non_thread_safe_call_in_test::register_lints(sess, lint_store);
    wrong_serialize_struct_arg::register_lints(sess, lint_store);
}
