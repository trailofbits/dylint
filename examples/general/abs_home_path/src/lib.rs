#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::{diagnostics::span_lint, match_def_path, path_def_id};
use dylint_internal::{home, paths};
use once_cell::unsync::OnceCell;
use rustc_ast::ast::LitKind;
use rustc_hir::{Closure, Expr, ExprKind, Item, ItemKind, Node, def_id::DefId};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;
use std::{
    collections::HashSet,
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
    /// This lint doesn't warn in build scripts (`build.rs`), as they often need to reference absolute paths.
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
    test_fns: HashSet<DefId>,
    home: OnceCell<Option<PathBuf>>,
}

impl<'tcx> LateLintPass<'tcx> for AbsHomePath {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        self.find_test_fns(cx);
    }

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

        if cx
            .tcx
            .hir()
            .parent_iter(expr.hir_id)
            .any(|(_id, node)| matches!(node, Node::Item(item) if self.is_test_item(item)))
        {
            return;
        }

        if let ExprKind::Lit(lit) = &expr.kind
            && let LitKind::Str(symbol, _) = lit.node
            && let path = Path::new(symbol.as_str())
            && path.is_absolute()
            && self
                .home
                .get_or_init(home::home_dir)
                .as_ref()
                .is_some_and(|dir| path.starts_with(dir))
        {
            span_lint(
                cx,
                ABS_HOME_PATH,
                Span::with_root_ctxt(expr.span.lo(), expr.span.hi()),
                "this path might not exist in production",
            );
        }
    }
}

// smoelius: The contents of this `impl` are based on:
// https://github.com/trailofbits/dylint/blob/3610f9b3ddd7847adeb00d3d33aa830a7db409c6/examples/general/non_thread_safe_call_in_test/src/late.rs#L87-L120
impl AbsHomePath {
    fn find_test_fns(&mut self, cx: &LateContext<'_>) {
        for item_id in cx.tcx.hir_free_items() {
            let item = cx.tcx.hir_item(item_id);
            // smoelius:
            // https://rustc-dev-guide.rust-lang.org/test-implementation.html?step-3-test-object-generation
            if let ItemKind::Const(ty, _, const_body_id) = item.kind
                && let Some(ty_def_id) = path_def_id(cx, ty)
                && match_def_path(cx, ty_def_id, &paths::TEST_DESC_AND_FN)
                && let const_body = cx.tcx.hir_body(const_body_id)
                && let ExprKind::Struct(_, fields, _) = const_body.value.kind
                && let Some(testfn) = fields.iter().find(|field| field.ident.as_str() == "testfn")
                // smoelius: Callee is `self::test::StaticTestFn`.
                && let ExprKind::Call(_, [arg]) = testfn.expr.kind
                && let ExprKind::Closure(Closure {
                    body: closure_body_id,
                    ..
                }) = arg.kind
                && let closure_body = cx.tcx.hir_body(*closure_body_id)
                // smoelius: Callee is `self::test::assert_test_result`.
                && let ExprKind::Call(_, [arg]) = closure_body.value.kind
                // smoelius: Callee is test function.
                && let ExprKind::Call(callee, _) = arg.kind
                && let Some(callee_def_id) = path_def_id(cx, callee)
            {
                // smoelius: Record both the `TestDescAndFn` and the test function.
                self.test_fns.insert(item.owner_id.to_def_id());
                self.test_fns.insert(callee_def_id);
            }
        }
    }

    fn is_test_item(&self, item: &Item) -> bool {
        self.test_fns
            .iter()
            .any(|&def_id| item.owner_id.to_def_id() == def_id)
    }
}

#[test]
fn ui() {
    use std::{
        io::{Write, stderr},
        path::Path,
    };

    // smoelius: On GitHub, `dylint` is stored on the D drive, not in the user's home directory on
    // the C drive.
    if let Some(home) = home::home_dir()
        && !Path::new(env!("CARGO_MANIFEST_DIR")).starts_with(home)
    {
        #[expect(clippy::explicit_write)]
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
