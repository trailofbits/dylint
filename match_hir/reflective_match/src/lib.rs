#![feature(rustc_private)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use match_hir::{
    __hir_ids_from_span_untyped as hir_ids_from_span_untyped, __snippet_opt as snippet_opt, Error,
    Pattern,
};
use rustc_hir::{Expr, ExprKind, HirId};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::TyCtxt;
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Symbol;
use std::str::FromStr;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
struct EmptySnippet;

impl std::fmt::Display for EmptySnippet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("empty snippet")
    }
}

#[derive(Debug, ThisError)]
struct Mismatch {
    snippet: String,
    expected: String,
    actual: String,
}

impl Mismatch {
    fn new(tcx: TyCtxt, snippet: String, expected: HirId, actual: HirId) -> Self {
        if enabled("VERBOSE") {
            Self {
                snippet,
                expected: format!("{:#?}", tcx.hir_node(expected)),
                actual: format!("{:#?}", tcx.hir_node(actual)),
            }
        } else {
            Self {
                snippet,
                expected: expected.to_string(),
                actual: actual.to_string(),
            }
        }
    }
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "`HirId`s do not match
     snippet: {:?}
    expected: {}
      actual: {}",
            self.snippet, self.expected, self.actual
        ))
    }
}

#[allow(clippy::no_mangle_with_rust_abi)]
#[no_mangle]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    let wildcard = std::env::var("WILDCARD").ok();
    let mut lock = sess.psess.env_depinfo.lock();
    lock.insert((
        Symbol::intern("WILDCARD"),
        wildcard.as_deref().map(Symbol::intern),
    ));

    lint_store.register_lints(&[REFLECTIVE_MATCH]);
    lint_store.register_late_pass(move |_| {
        Box::<_>::new(ReflectiveMatch::new(is_enabled_value(wildcard.clone())))
    });
}

declare_lint! {
    pub REFLECTIVE_MATCH,
    Warn,
    "match expressions against themselves using `match_hir`"
}

#[derive(Default)]
struct ReflectiveMatch {
    wildcard: bool,
    ignored: Vec<rustc_span::Span>,
    matched: Vec<rustc_span::Span>,
    errors: Vec<match_hir::Error>,
}

impl_lint_pass!(ReflectiveMatch => [REFLECTIVE_MATCH]);

impl ReflectiveMatch {
    fn new(wildcard: bool) -> Self {
        Self {
            wildcard,
            ..Default::default()
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for ReflectiveMatch {
    fn check_expr_post(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // smoelius: Ignore `DropTemps`.
        if matches!(expr.kind, ExprKind::DropTemps(_)) {
            self.ignored.push(expr.span);
            return;
        }

        // smoelius: `hir_ids_from_span_untyped` ignores "no-location" spans (i.e., spans with no
        // associated source file) and "from expansion" spans.
        if !hir_ids_from_span_untyped(cx, expr.span).contains(&expr.hir_id) {
            self.ignored.push(expr.span);
            return;
        };

        // smoelius: If any of `expr`'s ancestors have the same span as `expr`, ignore `expr`.
        if cx
            .tcx
            .hir_parent_id_iter(expr.hir_id)
            .any(|ancestor_id| cx.tcx.hir_span(ancestor_id) == expr.span)
        {
            self.ignored.push(expr.span);
            return;
        };

        let snippet = snippet_opt(cx, expr.span).unwrap();

        if snippet.is_empty() {
            self.errors.push(Error::other(expr.span, EmptySnippet));
            return;
        }

        let result = if self.wildcard {
            let pattern = Pattern::from_str("#(_)").unwrap();
            let mut result = pattern.matches(cx, expr);
            if let Ok(hir_ids) = &result {
                let &[hir_id] = hir_ids.as_slice() else {
                    panic!();
                };
                if expr.hir_id != hir_id {
                    result = Err(Error::other(
                        expr.span,
                        Mismatch::new(cx.tcx, snippet, expr.hir_id, hir_id),
                    ));
                }
            }
            result
        } else {
            let pattern = Pattern::from_str(&snippet).unwrap();
            let result = pattern.matches(cx, expr);
            if let Ok(hir_ids) = &result {
                assert!(hir_ids.is_empty());
            }
            result
        };

        match result {
            Ok(_) => self.matched.push(expr.span),
            Err(error) => {
                self.errors.push(error);
            }
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        eprintln!("name: {:?}", cx.sess().opts.crate_name.as_ref().unwrap());
        if enabled("VERBOSE") {
            eprintln!("ignored: {}", self.ignored.len());
            eprintln!("matched: {}", self.matched.len());
        }
        eprintln!("errors: {}", self.errors.len());
        for error in &self.errors {
            eprintln!("{error}");
        }
    }
}

fn enabled(key: &str) -> bool {
    is_enabled_value(std::env::var(key).ok())
}

fn is_enabled_value<T: AsRef<str>>(value: Option<T>) -> bool {
    value.map_or(false, |value| value.as_ref() != "0")
}
