#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::{AttrStyle, Crate, MetaItem, MetaItemKind};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_span::sym;

dylint_linting::declare_early_lint! {
    /// ### What it does
    ///
    /// Checks for use of `#![allow(...)]` at the crate level.
    ///
    /// ### Why is this bad?
    ///
    /// Such uses cannot be overridden with `--warn` or `--deny` from the command line. They _can_
    /// be overridden with `--force-warn` or `--forbid`, but one must know the `#![allow(...)]`
    /// are present to use these unconventional options.
    ///
    /// ### Example
    ///
    /// ```rust
    /// #![allow(clippy::assertions_on_constants)] // in code
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// // Allow `clippy::assertions-on-constants` in Cargo.toml. See:
    /// // - https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section
    /// // - https://doc.rust-lang.org/clippy/configuration.html#lints-section-in-cargotoml
    /// ```
    pub CRATE_WIDE_ALLOW,
    Warn,
    "use of `#![allow(...)]` at the crate level"
}

impl EarlyLintPass for CrateWideAllow {
    fn check_crate(&mut self, cx: &EarlyContext, krate: &Crate) {
        for attr in &krate.attrs {
            assert_eq!(AttrStyle::Inner, attr.style);
            if attr.has_name(sym::allow)
                && let Some([arg]) = attr.meta_item_list().as_deref()
                && let Some(MetaItem {
                    path,
                    kind: MetaItemKind::Word,
                    ..
                }) = arg.meta_item()
            {
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
                    format!("silently overrides `--warn {path}` and `--deny {path}`"),
                    None,
                    format!("allow `{path}` in Cargo.toml"),
                );
            }
        }
    }
}

#[cfg(test)]
mod test {
    use assert_cmd::{Command, assert::Assert};
    use dylint_internal::{env, testing::cargo_dylint};
    use predicates::prelude::*;
    use std::{
        path::PathBuf,
        sync::{LazyLock, Mutex, MutexGuard},
    };

    fn mutex<T: maybe_return::MaybeReturn<MutexGuard<'static, ()>>>() -> T::Output {
        static MUTEX: Mutex<()> = Mutex::new(());

        let lock = MUTEX.lock().unwrap();

        // smoelius: Ensure the `clippy` component is installed.
        Command::new("rustup")
            .args(["component", "add", "clippy"])
            .assert()
            .success();

        T::maybe_return(lock)
    }

    #[test]
    fn ui() {
        let _lock = mutex::<maybe_return::Yes>();

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
    // - `dylint_testing::ui_test_example` above sets `DYLINT_LIBRARY_PATH`. Having this environment
    //   variable set causes "found multiple libraries" errors when Dylint is run directly. Hence,
    //   the variable must be unset before Dylint can be run directly.
    // - Setting `RUSTFLAGS` forces `cargo check` to be re-run. Unfortunately, this also forces
    //   `cargo-dylint` to be rebuilt, which causes problems on Windows, hence the need for the
    //   mutex.
    // smoelius: Invoking `cargo-dylint` directly by path, rather than through `cargo run`, avoids
    // the rebuilding problem. But oddly enough, the tests are faster with the mutex than without.
    // smoelius: The real reason this test is slow is that setting `RUSTFLAGS` causes the metadata
    // entries to be rebuilt. Running `clippy` once and passing `--no-build` thereafter avoids this
    // problem.
    // smoelius: Metadata entries are no longer rebuilt when `RUSTFLAGS` changes.

    fn test(rustflags: &str, assert: impl Fn(Assert) -> Assert) {
        static CARGO_DYLINT_PATH: LazyLock<PathBuf> =
            LazyLock::new(|| cargo_dylint("../../..").unwrap());

        let _lock = mutex::<maybe_return::Yes>();

        let cargo_dylint = |example_rustflags: Option<&str>| {
            let mut command = Command::new(&*CARGO_DYLINT_PATH);
            command
                .env_remove(env::DYLINT_LIBRARY_PATH)
                .args(["dylint", "--lib", "clippy"]);
            if let Some(rustflags) = example_rustflags {
                command.env(
                    env::RUSTFLAGS,
                    "--cfg no_dev_dependencies ".to_owned() + rustflags,
                );
                command.args(["--no-build"]);
            }
            command.args(["--", "--examples"]);
            command.assert()
        };

        cargo_dylint(None).success();

        assert(cargo_dylint(Some(rustflags)));
    }

    const ASSERTIONS_ON_CONSTANTS_WARNING: &str =
        "`assert!(true)` will be optimized out by the compiler";

    #[test]
    fn premise_manifest_sanity() {
        mutex::<maybe_return::No>();

        let mut command = Command::new("cargo");
        command.args(["clippy"]);
        command.current_dir("ui_manifest");
        command
            .assert()
            .success()
            .stderr(predicate::str::contains(ASSERTIONS_ON_CONSTANTS_WARNING).not());
    }

    /// Verify that `allow`ing a lint in the manifest does not silently override `--warn`.
    #[test]
    fn premise_manifest_warn() {
        mutex::<maybe_return::No>();

        let mut command = Command::new("cargo");
        command.args(["clippy", "--", "--warn=clippy::assertions-on-constants"]);
        command.current_dir("ui_manifest");
        command
            .assert()
            .success()
            .stderr(predicate::str::contains(ASSERTIONS_ON_CONSTANTS_WARNING));
    }

    /// Verify that `allow`ing a lint in the manifest does not silently override `--deny`.
    #[test]
    fn premise_manifest_deny() {
        mutex::<maybe_return::No>();

        let mut command = Command::new("cargo");
        command.args(["clippy", "--", "--deny=clippy::assertions-on-constants"]);
        command.current_dir("ui_manifest");
        command
            .assert()
            .failure()
            .stderr(predicate::str::contains(ASSERTIONS_ON_CONSTANTS_WARNING));
    }

    mod maybe_return {
        pub trait MaybeReturn<T> {
            type Output;
            fn maybe_return(value: T) -> Self::Output;
        }

        pub struct Yes;

        pub struct No;

        impl<T> MaybeReturn<T> for Yes {
            type Output = T;
            fn maybe_return(value: T) -> Self::Output {
                value
            }
        }

        impl<T> MaybeReturn<T> for No {
            type Output = ();
            fn maybe_return(_value: T) -> Self::Output {}
        }
    }
}
