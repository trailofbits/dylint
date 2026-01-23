#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::{diagnostics::span_lint, is_in_test};
use once_cell::unsync::OnceCell;
use rustc_ast::ast::LitKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;
use std::{
    env::home_dir,
    path::{Path, PathBuf},
};

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Checks for string literals that are absolute paths into the user's home directory, e.g.,
    /// `env!("CARGO_MANIFEST_DIR")`.
    ///
    /// ### Why is this bad?
    ///
    /// The path might not exist when the code is used in production.
    ///
    /// ### Known problems
    ///
    /// The lint does not apply inside macro arguments. So false negatives could result.
    ///
    /// ### Note
    ///
    /// This lint doesn't warn in build scripts (`build.rs`) or test contexts, as they often need to reference absolute paths.
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn main() {
    ///     let path = option_env!("CARGO");
    ///     println!("{:?}", path);
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// fn main() {
    ///     let path = std::env::var("CARGO");
    ///     println!("{:?}", path);
    /// }
    /// ```
    pub ABS_HOME_PATH,
    Warn,
    "string literals that are absolute paths into the user's home directory",
    AbsHomePath::default()
}

#[derive(Default)]
pub struct AbsHomePath {
    home: OnceCell<Option<PathBuf>>,
}

impl<'tcx> LateLintPass<'tcx> for AbsHomePath {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr) {
        // Skip build scripts
        if cx
            .sess()
            .opts
            .crate_name
            .as_ref()
            .is_some_and(|crate_name| crate_name == "build_script_build")
        {
            return;
        }

        // Skip expressions inside test functions
        if is_in_test(cx.tcx, expr.hir_id) {
            return;
        }

        if let ExprKind::Lit(lit) = &expr.kind
            && let LitKind::Str(symbol, _) = lit.node
            && let path = Path::new(symbol.as_str())
            && path.is_absolute()
            && self
                .home
                .get_or_init(home_dir)
                .as_ref()
                .is_some_and(|dir| path.starts_with(dir))
        {
            span_lint(
                cx,
                ABS_HOME_PATH,
                get_nearest_root_span(expr.span),
                "this path might not exist in production",
            );
        }
    }
}

fn get_nearest_root_span(mut span: Span) -> Span {
    while !span.ctxt().is_root() {
        span = span.source_callsite();
    }
    span
}

#[test]
fn ui() {
    use std::{
        io::{Write, stderr},
        path::Path,
    };

    // smoelius: On GitHub, `dylint` is stored on the D drive, not in the user's home directory on
    // the C drive.
    if let Some(home) = home_dir()
        && !Path::new(env!("CARGO_MANIFEST_DIR")).starts_with(home)
    {
        writeln!(
            stderr(),
            "Skipping `ui` test as repository is not stored in the user's home directory"
        )
        .unwrap();
        return;
    }

    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui")
        .rustc_flags(["--test"])
        .run();
}

// Combined test for context allowances (build script and test context)
#[test]
fn context_allowance() {
    use dylint_internal::CommandExt;
    use std::{
        io::{Write, stderr},
        path::Path,
        process::{Command, Output},
    };

    struct TestCase {
        context_name: &'static str,
        manifest_path: &'static str,
        deny_warnings: bool,
        assert_fn: fn(&Output),
    }

    let test_cases = [
        TestCase {
            context_name: "build script",
            manifest_path: "ui_build_script/Cargo.toml",
            deny_warnings: true,
            assert_fn: |output: &Output| {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(
                    !stderr.contains("this path might not exist in production"),
                    "The abs_home_path lint was incorrectly triggered in a build script: {stderr}"
                );
            },
        },
        TestCase {
            context_name: "test context",
            manifest_path: "ui_test/Cargo.toml",
            deny_warnings: false,
            assert_fn: |output: &Output| {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // We expect warnings for non-test functions but not for test functions
                assert!(
                    stderr.contains("this path might not exist in production"),
                    "Expected the abs_home_path lint to trigger for non-test functions in test context: {stderr}"
                );
                // Verify the test detected non-test functions correctly
                assert!(
                    stderr.contains("src/main.rs:8:13") && stderr.contains("src/main.rs:9:13"),
                    "Expected warning in non-test function in test context: {stderr}"
                );
                // Verify that test functions weren't warned about
                assert!(
                    stderr.lines().all(|line| !line.contains("src/main.rs")
                        || line.contains("src/main.rs:8:13")
                        || line.contains("src/main.rs:9:13")),
                    "The abs_home_path lint should not trigger for lines other than src/main.rs:8:13 or src/main.rs:9:13: {stderr}"
                );
            },
        },
    ];

    // Skip tests if repository is not stored in the user's home directory
    if let Some(home) = home_dir()
        && !Path::new(env!("CARGO_MANIFEST_DIR")).starts_with(home)
    {
        writeln!(
            stderr(),
            "Skipping `context_allowance` tests as repository is not stored in the user's home directory"
        )
        .unwrap();
        return;
    }

    let cargo_dylint = dylint_internal::testing::cargo_dylint("../../..").unwrap();
    for case in test_cases {
        println!("Testing {} allowance...", case.context_name);
        let mut command = Command::new(&cargo_dylint);
        if case.deny_warnings {
            command.env("DYLINT_RUSTFLAGS", "--deny warnings");
        }
        let output = command
            .args([
                "dylint",
                "--manifest-path",
                case.manifest_path,
                "--path",
                env!("CARGO_MANIFEST_DIR"),
                "--",
                "--all-targets",
            ])
            .logged_output(true)
            .unwrap_or_else(|error| panic!("Failed to execute cargo-dylint command: {error}"));

        (case.assert_fn)(&output);
    }
}
