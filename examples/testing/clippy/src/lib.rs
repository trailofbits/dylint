#![feature(rustc_private)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_lint;
extern crate rustc_session;

use declare_clippy_lint::LintListBuilder;
use dylint_internal::env;
use std::env::{remove_var, set_var};

/// All of the Clippy lints as a Dylint library
#[expect(clippy::no_mangle_with_rust_abi)]
#[unsafe(no_mangle)]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    if let Ok(clippy_disable_docs_links) = env::var(env::CLIPPY_DISABLE_DOCS_LINKS)
        && let Ok(val) = serde_json::from_str::<Option<String>>(&clippy_disable_docs_links)
        && let Some(val) = val
    {
        unsafe {
            set_var(env::CLIPPY_DISABLE_DOCS_LINKS, val);
        }
    } else {
        unsafe {
            remove_var(env::CLIPPY_DISABLE_DOCS_LINKS);
        }
    }

    let mut list_builder = LintListBuilder::default();
    list_builder.insert(clippy_lints::declared_lints::LINTS);
    list_builder.register(lint_store);

    let conf_path = clippy_config::lookup_conf_file();
    let conf = clippy_config::Conf::read(sess, &conf_path);
    clippy_lints::register_lint_passes(lint_store, conf);
}
