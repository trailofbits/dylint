#![feature(rustc_private)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_lint;
extern crate rustc_session;

#[expect(clippy::no_mangle_with_rust_abi)]
#[unsafe(no_mangle)]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // smoelius: Please keep the following `register_lints` calls sorted by crate name.
    arg_iter::register_lints(sess, lint_store);
    commented_out_code::register_lints(sess, lint_store);
    dir_entry_path_file_name::register_lints(sess, lint_store);
    escaping_doc_link::register_lints(sess, lint_store);
    inconsistent_struct_pattern::register_lints(sess, lint_store);
    local_ref_cell::register_lints(sess, lint_store);
    nonexistent_path_in_comment::register_lints(sess, lint_store);
    redundant_reference::register_lints(sess, lint_store);
    unnamed_constant::register_lints(sess, lint_store);
    unnecessary_borrow_mut::register_lints(sess, lint_store);
    unnecessary_conversion_for_trait::register_lints(sess, lint_store);
}
