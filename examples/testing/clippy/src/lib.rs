#![feature(rustc_private)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_lint;
extern crate rustc_session;

use dylint_internal::env;
use std::env::{remove_var, set_var};

/// All of the Clippy lints as a Dylint library
#[allow(clippy::no_mangle_with_rust_abi)]
#[no_mangle]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    if let Ok(clippy_disable_docs_links) = env::var(env::CLIPPY_DISABLE_DOCS_LINKS) {
        if let Ok(val) = serde_json::from_str::<Option<String>>(&clippy_disable_docs_links) {
            if let Some(val) = val {
                set_var(env::CLIPPY_DISABLE_DOCS_LINKS, val);
            } else {
                remove_var(env::CLIPPY_DISABLE_DOCS_LINKS);
            }
        }
    }

    let conf_path = clippy_config::lookup_conf_file();
    let conf = clippy_config::Conf::read(sess, &conf_path);
    clippy_lints::register_lints(lint_store, conf);
    clippy_lints::register_pre_expansion_lints(lint_store, conf);
    clippy_lints::register_renamed(lint_store);
}
