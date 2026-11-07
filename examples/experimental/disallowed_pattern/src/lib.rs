#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use anyhow::{Result, anyhow};
use clippy_utils::diagnostics::span_lint;
use match_hir::ErrorKind;
use paste::paste;
use rustc_hir::HirId;
use rustc_lint::{LateContext, LateLintPass};
use std::path::Path;

mod config;
use config::{Config, Type};

mod pattern;
use pattern::{CallbackDir, Pattern};

use crate::pattern::{CompiledPattern, UncompiledPattern};

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Checks for code patterns a user wishes to forbid.
    ///
    /// ### Why is this bad?
    ///
    /// It depends on the code pattern and the user's reason for wanting to forbid it.
    pub DISALLOWED_PATTERN,
    Warn,
    "code patterns a user wishes to forbid",
    DisallowedPattern::new()
}

struct DisallowedPattern {
    config: Config<CompiledPattern>,
}

impl DisallowedPattern {
    pub fn new() -> Self {
        let config: Config<UncompiledPattern> =
            dylint_linting::config_or_default(env!("CARGO_PKG_NAME"));
        Self {
            config: config.compile(),
        }
    }
}

macro_rules! impl_check {
    ($ty:ident) => {
        paste! {
            fn [< check_ $ty >](&mut self, cx: &LateContext<'tcx>, $ty: &'tcx rustc_hir::[< $ty:camel >]) {
                let patterns = self.config.get_slice(Type::[< $ty:camel >]);
                Self::check_with_patterns(cx, $ty, patterns);
            }
        }
    };
}

impl<'tcx> LateLintPass<'tcx> for DisallowedPattern {
    impl_check!(arm);
    impl_check!(block);
    impl_check!(expr);
    impl_check!(stmt);
}

impl DisallowedPattern {
    fn check_with_patterns<T>(
        cx: &LateContext<'_>,
        hir_node: &T,
        patterns: Option<&[Pattern<match_hir::Pattern, CallbackDir>]>,
    ) where
        T: match_hir::HirNode + match_hir::HirToSyn,
    {
        let Some(patterns) = patterns else {
            return;
        };
        let hir_id = hir_node.hir_id();
        for Pattern {
            pattern,
            predicate,
            callback,
            dependencies: _,
            reason,
        } in patterns
        {
            match pattern.matches_hir_id::<T::Syn>(cx, hir_id) {
                Ok(hir_ids) => {
                    if let Some(callback_dir) = callback {
                        assert!(!predicate.is_some());
                        unsafe { call::<()>(cx, &callback_dir.lib_path(), &hir_ids) }.unwrap();
                        return;
                    }
                    if let Some(predicate_dir) = predicate
                        && !unsafe { call::<bool>(cx, &predicate_dir.lib_path(), &hir_ids) }
                            .unwrap()
                    {
                        return;
                    }

                    let span = cx.tcx.hir_span(hir_id);
                    let msg = reason.clone().unwrap_or(String::from("disallowed pattern"));
                    span_lint(cx, DISALLOWED_PATTERN, span, msg);
                }
                Err(error) => {
                    debug_assert!(matches!(
                        error.kind(),
                        ErrorKind::NoSource | ErrorKind::NoMatch | ErrorKind::NoHirId { .. }
                    ));
                }
            }
        }
    }
}

type Callback<T> = fn(&LateContext<'_>, &[HirId]) -> T;

unsafe fn call<T>(cx: &LateContext, path: &Path, hir_ids: &[HirId]) -> Result<T> {
    let lib = unsafe { libloading::Library::new(path) }?;
    let func = unsafe { lib.get::<Callback<T>>(b"callback") }.map_err(|error| {
        anyhow!(
            "could not find callback in `{}`: {error}",
            path.to_string_lossy()
        )
    })?;
    let result = func(cx, hir_ids);
    Ok(result)
}

#[test]
fn ui_predicate() {
    let toml = r##"
[[disallowed_pattern.patterns]]
pattern = "#(_) ( #(_) )"
predicate = """
    |cx: &LateContext<'tcx>, callee: &Expr<'tcx>, arg: &Expr<'tcx>| {
        extern crate rustc_ast;
        extern crate rustc_middle;
        use clippy_utils::paths::{PathNS, lookup_path_str};
        let callee_ty = cx.typeck_results().expr_ty(callee);
        if let &rustc_middle::ty::FnDef(def_id, _) = callee_ty.kind()
            && (lookup_path_str(cx.tcx, PathNS::Value, "std::env::remove_var") == [def_id]
                || lookup_path_str(cx.tcx, PathNS::Value, "std::env::var") == [def_id])
            && let rustc_hir::ExprKind::Lit(lit) = arg.kind
            && matches!(lit.node, rustc_ast::ast::LitKind::Str(_, _))
        {
            true
        } else {
            false
        }
    }
"""
reason = "referring to an environment variable with a string literal is error prone"

[[disallowed_pattern.patterns]]
pattern = "#(_) ( #(_), #(_) )"
predicate = """
    |cx: &LateContext<'tcx>, callee: &Expr<'tcx>, arg: &Expr<'tcx>, _: &Expr<'tcx>| {
        extern crate rustc_ast;
        extern crate rustc_middle;
        use clippy_utils::paths::{PathNS, lookup_path_str};
        let callee_ty = cx.typeck_results().expr_ty(callee);
        if let &rustc_middle::ty::FnDef(def_id, _) = callee_ty.kind()
            && lookup_path_str(cx.tcx, PathNS::Value, "std::env::set_var") == [def_id]
            && let rustc_hir::ExprKind::Lit(lit) = arg.kind
            && matches!(lit.node, rustc_ast::ast::LitKind::Str(_, _))
        {
            true
        } else {
            false
        }
    }
"""
"##;

    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui_predicate")
        .dylint_toml(toml)
        .run();
}

#[test]
fn ui_callback() {
    let toml = r##"
[[disallowed_pattern.patterns]]
pattern = "#(_) ( #(_) )"
callback = """
    |cx: &LateContext<'tcx>, callee: &Expr<'tcx>, arg: &Expr<'tcx>| {
        extern crate rustc_ast;
        extern crate rustc_middle;
        use clippy_utils::paths::{PathNS, lookup_path_str};
        let callee_ty = cx.typeck_results().expr_ty(callee);
        if let &rustc_middle::ty::FnDef(def_id, _) = callee_ty.kind()
            && (lookup_path_str(cx.tcx, PathNS::Value, "std::env::remove_var") == [def_id]
                || lookup_path_str(cx.tcx, PathNS::Value, "std::env::var") == [def_id])
            && let rustc_hir::ExprKind::Lit(lit) = arg.kind
            && let rustc_ast::LitKind::Str(ident, _) = lit.node
        {
            clippy_utils::diagnostics::span_lint_and_help(
                cx,
                DISALLOWED_PATTERN,
                arg.span,
                "referring to an environment variable with a string literal is error prone",
                None,
                format!("define a constant `{ident}` and use that instead"),
            );
        }
    }
"""

[[disallowed_pattern.patterns]]
pattern = "#(_) ( #(_), #(_) )"
callback = """
    |cx: &LateContext<'tcx>, callee: &Expr<'tcx>, arg: &Expr<'tcx>, _: &Expr<'tcx>| {
        extern crate rustc_ast;
        extern crate rustc_middle;
        use clippy_utils::paths::{PathNS, lookup_path_str};
        let callee_ty = cx.typeck_results().expr_ty(callee);
        if let &rustc_middle::ty::FnDef(def_id, _) = callee_ty.kind()
            && lookup_path_str(cx.tcx, PathNS::Value, "std::env::set_var") == [def_id]
            && let rustc_hir::ExprKind::Lit(lit) = arg.kind
            && let rustc_ast::LitKind::Str(ident, _) = lit.node
        {
            clippy_utils::diagnostics::span_lint_and_help(
                cx,
                DISALLOWED_PATTERN,
                arg.span,
                "referring to an environment variable with a string literal is error prone",
                None,
                format!("define a constant `{ident}` and use that instead"),
            );
        }
    }
"""
"##;

    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui_callback")
        .dylint_toml(toml)
        .run();
}
