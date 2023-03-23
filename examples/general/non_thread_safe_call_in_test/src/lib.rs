#![feature(rustc_private)]
#![recursion_limit = "256"]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod blacklist;
mod late;
mod pre_expansion;

#[allow(clippy::no_mangle_with_rust_abi)]
#[no_mangle]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[pre_expansion::NON_THREAD_SAFE_CALL_IN_TEST_PRE_EXPANSION]);
    lint_store
        .register_pre_expansion_pass(|| Box::<pre_expansion::NonThreadSafeCallInTest>::default());

    lint_store.register_lints(&[late::NON_THREAD_SAFE_CALL_IN_TEST]);
    lint_store.register_late_pass(|_| Box::<late::NonThreadSafeCallInTest>::default());

    if !sess.opts.test {
        sess.warn("`non_thread_safe_call_in_test` is unlikely to be effective as `--test` was not passed to rustc");
    }
}

#[test]
fn ui_pre_expansion() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui_pre_expansion"),
    );
}

#[test]
fn ui_late() {
    dylint_testing::ui::Test::examples(env!("CARGO_PKG_NAME"))
        .rustc_flags(["--test"])
        .run();
}
