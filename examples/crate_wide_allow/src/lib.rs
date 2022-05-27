#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_help;
use if_chain::if_chain;
use rustc_ast::{AttrStyle, Crate, MetaItem, MetaItemKind};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_span::sym;

dylint_linting::declare_early_lint! {
    /// **What it does:** Checks for use of `#![allow(...)]` at the crate level.
    ///
    /// **Why is this bad?** Such uses cannot be overridden with `--warn` or `--deny` from the
    /// command line. They *can* be overridden with `--force-warn` or `--forbid`, but one must
    /// know the `#![allow(...)]` are present to use these unconventional options.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// Bad:
    /// ```rust
    /// #![allow(clippy::assertions_on_constants)] // in code
    /// ```
    ///
    /// Good:
    /// ```rust
    /// // Pass `--allow clippy::assertions-on-constants` on the command line.
    /// ```
    pub CRATE_WIDE_ALLOW,
    Warn,
    "use of `#![allow(...)]` at the crate level"
}

impl EarlyLintPass for CrateWideAllow {
    fn check_crate(&mut self, cx: &EarlyContext, krate: &Crate) {
        for attr in &krate.attrs {
            assert_eq!(attr.style, AttrStyle::Inner);
            if_chain! {
                if attr.has_name(sym::allow);
                if let Some([arg]) = attr.meta_item_list().as_deref();
                if let Some(MetaItem {
                    path,
                    kind: MetaItemKind::Word,
                    ..
                }) = arg.meta_item();
                then {
                    let path = path
                        .segments
                        .iter()
                        .map(|segment| segment.ident.as_str())
                        .collect::<Vec<_>>()
                        .join("::")
                        .replace('_', "-");
                    span_lint_and_help(
                        cx,
                        CRATE_WIDE_ALLOW,
                        attr.span,
                        &format!("silently overrides `--warn {}` and `--deny {}`", path, path),
                        None,
                        &format!("pass `--allow {}` on the command line", path),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use assert_cmd::{assert::Assert, Command};
    use dylint_internal::env;
    use lazy_static::lazy_static;
    use std::{path::Path, sync::Mutex};

    lazy_static! {
        static ref MUTEX: Mutex<()> = Mutex::new(());
    }

    #[test]
    fn ui() {
        let _lock = MUTEX.lock().unwrap();

        dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
    }

    #[test]
    fn premise_warn() {
        test("--warn=clippy::assertions-on-constants", Assert::success);
    }

    #[test]
    fn premise_deny() {
        test("--deny=clippy::assertions-on-constants", Assert::success);
    }

    #[test]
    fn premise_forbid() {
        test("--forbid=clippy::assertions-on-constants", Assert::failure);
    }

    // smoelius: Here is why the below uses of `env_remove` and `env` are needed:
    // * `dylint_testing::ui_test_example` above sets `DYLINT_LIBRARY_PATH`. Having this environment
    //   variable set causes "found multiple libraries" errors when Dylint is run directly. Hence,
    //   the variable must be unset before Dylint can be run directly.
    // * Setting `RUSTFLAGS` forces `cargo check` to be re-run. Unfortunately, this also forces
    //   `cargo-dylint` to be rebuilt, which causes problems on Windows, hence the need for the
    //   mutex.

    fn test(rustflags: &str, check: impl Fn(Assert) -> Assert) {
        let _lock = MUTEX.lock().unwrap();

        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("Cargo.toml");

        check(
            Command::new("cargo")
                .env_remove(env::DYLINT_LIBRARY_PATH)
                .env(env::RUSTFLAGS, rustflags)
                .args(&[
                    "run",
                    "--manifest-path",
                    &manifest.to_string_lossy(),
                    "--bin",
                    "cargo-dylint",
                    "--",
                    "dylint",
                    "clippy",
                    "--",
                    "--examples",
                ])
                .assert(),
        );
    }
}
