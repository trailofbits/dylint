#![feature(rustc_private)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_ast;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

mod env_cargo_path;

#[doc(hidden)]
#[no_mangle]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[env_cargo_path::ENV_CARGO_PATH]);
    lint_store.register_pre_expansion_pass(|| Box::new(env_cargo_path::EnvCargoPath::default()));
}

#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}
