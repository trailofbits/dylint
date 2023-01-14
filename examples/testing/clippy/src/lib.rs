#![feature(rustc_private)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_lint;
extern crate rustc_session;

use dylint_internal::env;
use std::env::{remove_var, set_var};

/// All of the Clippy lints as a Dylint library
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

    // smoelius: FIXME: `Ok(None)` implies there is no `clippy.toml`.
    let conf = clippy_lints::read_conf(sess, &Ok(None));
    clippy_lints::register_plugins(lint_store, sess, &conf);
    clippy_lints::register_pre_expansion_lints(lint_store, sess, &conf);
    clippy_lints::register_renamed(lint_store);
}
