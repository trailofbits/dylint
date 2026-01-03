#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::{
    diagnostics::span_lint_and_sugg,
    path_to_local_id,
    source::snippet,
    ty::{is_copy, peel_and_count_ty_refs},
};
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_hir::{
    BindingMode, Body, ByRef, Expr, ExprKind, FnDecl, HirId, Node, Pat, PatKind, UnOp,
    def_id::LocalDefId,
    intravisit::{FnKind, Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{
    self,
    adjustment::{Adjust, PatAdjustment},
};
use rustc_span::Span;
use serde::Deserialize;
use std::{cmp::min, fmt::Write};

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Checks for patterns that could perform additional destructuring.
    ///
    /// ### Why is this bad?
    ///
    /// The use of destructuring patterns in closure parameters (for example) often leads to more
    /// concise closure bodies. Beyond that, the benefits of this lint are similar to those of
    /// [pattern-type-mismatch].
    ///
    /// ### Known problems
    ///
    /// - Currently only checks closure parameters (not, e.g., match patterns).
    /// - Currently only suggests destructuring references and tuples (not, e.g., arrays or
    ///   structs).
    /// - For the lint to suggest destructuring a reference, the idents involved must not use `ref`
    ///   annotations.
    ///
    /// ### Example
    ///
    /// ```rust
    /// let xs = [0, 1, 2];
    /// let ys = xs.iter().map(|x| *x == 0).collect::<Vec<_>>();
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// let xs = [0, 1, 2];
    /// let ys = xs.iter().map(|&x| x == 0).collect::<Vec<_>>();
    /// ```
    ///
    /// ### Configuration
    ///
    /// - `explicit_deref_check: bool` (default `true`): By default, `suboptimal_pattern` will not
    ///   suggest to destructure a reference unless it would eliminate at least one explicit
    ///   dereference. Setting `explicit_deref_check` to `false` disables this check.
    ///
    /// [pattern-type-mismatch]: https://rust-lang.github.io/rust-clippy/master/#pattern_type_mismatch
    pub SUBOPTIMAL_PATTERN,
    Warn,
    "patterns that could perform additional destructuring",
    SuboptimalPattern::new()
}

#[derive(Deserialize)]
struct Config {
    explicit_deref_check: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            explicit_deref_check: true,
        }
    }
}

struct SuboptimalPattern {
    config: Config,
}

impl SuboptimalPattern {
    pub fn new() -> Self {
        Self {
            config: dylint_linting::config_or_default(env!("CARGO_PKG_NAME")),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for SuboptimalPattern {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        fn_kind: FnKind<'tcx>,
        _: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _: LocalDefId,
    ) {
        if !matches!(fn_kind, FnKind::Closure) {
            return;
        }

        if span.from_expansion() {
            return;
        }

        let mut found = false;

        for param in body.params {
            if found {
                break;
            }

            param.pat.walk(|pat| {
                let pat_ty = if let Some([PatAdjustment { source, .. }, ..]) = cx
                    .typeck_results()
                    .pat_adjustments()
                    .get(pat.hir_id)
                    .map(Vec::as_slice)
                {
                    *source
                } else {
                    cx.typeck_results().node_type(pat.hir_id)
                };
                let (referent_ty, n_refs, _) = peel_and_count_ty_refs(pat_ty);

                let mut is_tuple = false;

                if let ty::Tuple(tys) = referent_ty.kind() {
                    is_tuple = true;

                    if let PatKind::Binding(BindingMode(ByRef::No, _), hir_id, ident, None) =
                        pat.kind
                        && let Some(projections) = exclusively_projected(cx.tcx, hir_id, body.value)
                    {
                        let tuple_pattern =
                            build_tuple_pattern(ident.name.as_str(), &projections, tys.len());
                        let pattern = format!(
                            "{:&>width$}{}",
                            "",
                            tuple_pattern,
                            width = if is_copy(cx, referent_ty) { n_refs } else { 0 }
                        );
                        span_lint_and_sugg(
                            cx,
                            SUBOPTIMAL_PATTERN,
                            pat.span,
                            "could destructure tuple",
                            "use something like",
                            pattern,
                            Applicability::HasPlaceholders,
                        );
                        found = true;
                        return false;
                    }
                }

                if !contains_wild(pat)
                    && let Some(hir_ids) = collect_non_ref_idents(pat)
                    && let Some(n_derefs) = exclusively_dereferenced(
                        self.config.explicit_deref_check,
                        cx,
                        hir_ids,
                        body.value,
                    )
                    && n_derefs > 0
                {
                    let snippet = snippet(cx, pat.span, "_");
                    let pattern = format!("{:&>width$}{}", "", snippet, width = n_derefs);
                    span_lint_and_sugg(
                        cx,
                        SUBOPTIMAL_PATTERN,
                        pat.span,
                        format!(
                            "could destructure reference{}",
                            if n_derefs > 1 { "s" } else { "" }
                        ),
                        "use",
                        pattern,
                        Applicability::HasPlaceholders,
                    );
                    found = true;
                    return false;
                }

                // smoelius: If the pattern is a tuple with (possibly implicit) outer references, do
                // not walk its children. (Some of the suggestions this lint was previously offering
                // were incorrect.)
                !is_tuple || n_refs == 0
            });
        }
    }
}

fn exclusively_projected<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    hir_id: HirId,
    expr: &'tcx Expr<'tcx>,
) -> Option<FxHashSet<usize>> {
    let mut visitor = ProjectionVisitor {
        tcx,
        hir_id,
        projections: Some(FxHashSet::default()),
    };
    visitor.visit_expr(expr);
    visitor.projections
}

struct ProjectionVisitor<'tcx> {
    tcx: ty::TyCtxt<'tcx>,
    hir_id: HirId,
    projections: Option<FxHashSet<usize>>,
}

impl<'tcx> Visitor<'tcx> for ProjectionVisitor<'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if path_to_local_id(expr, self.hir_id) {
            let node = self.tcx.parent_hir_node(expr.hir_id);
            if let Node::Expr(Expr {
                kind: ExprKind::Field(_, ident),
                ..
            }) = node
                && let Ok(projection) = ident.name.as_str().parse::<usize>()
            {
                self.projections
                    .as_mut()
                    .map(|projections| projections.insert(projection));
            } else {
                self.projections = None;
            }
        }
        walk_expr(self, expr);
    }
}

fn build_tuple_pattern(ident: &str, projections: &FxHashSet<usize>, size: usize) -> String {
    let mut buf = "(".to_owned();
    for i in 0..size {
        if projections.contains(&i) {
            write!(buf, "{ident}_{i}").unwrap();
        } else {
            write!(buf, "_").unwrap();
        }
        if size == 1 {
            write!(buf, ",").unwrap();
        } else if i + 1 < size {
            write!(buf, ", ").unwrap();
        }
    }
    write!(buf, ")").unwrap();
    buf
}

fn contains_wild(pat: &Pat<'_>) -> bool {
    let mut found = false;
    pat.walk(|pat| {
        found |= matches!(pat.kind, PatKind::Wild);
        !found
    });
    found
}

fn collect_non_ref_idents(pat: &Pat<'_>) -> Option<FxHashSet<HirId>> {
    let mut hir_ids = Some(FxHashSet::default());
    pat.walk(|pat| {
        if let PatKind::Binding(annotation, _, _, _) = pat.kind {
            match annotation {
                BindingMode(ByRef::No, _) => {
                    hir_ids.as_mut().map(|hir_ids| hir_ids.insert(pat.hir_id));
                }
                BindingMode(ByRef::Yes(..), _) => {
                    hir_ids = None;
                }
            }
        }
        hir_ids.is_some()
    });
    hir_ids
}

fn exclusively_dereferenced<'tcx>(
    explicit_deref_check: bool,
    cx: &LateContext<'tcx>,
    hir_ids: FxHashSet<HirId>,
    expr: &'tcx Expr<'tcx>,
) -> Option<usize> {
    let mut visitor = DereferenceVisitor {
        cx,
        hir_ids,
        n_derefs: usize::MAX,
        explicit_deref: !explicit_deref_check,
    };
    visitor.visit_expr(expr);
    if visitor.n_derefs < usize::MAX && visitor.explicit_deref {
        Some(visitor.n_derefs)
    } else {
        None
    }
}

struct DereferenceVisitor<'cx, 'tcx> {
    cx: &'cx LateContext<'tcx>,
    hir_ids: FxHashSet<HirId>,
    n_derefs: usize,
    explicit_deref: bool,
}

impl<'tcx> Visitor<'tcx> for DereferenceVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if self
            .hir_ids
            .iter()
            .any(|&hir_id| path_to_local_id(expr, hir_id))
        {
            let (n_derefs, explicit_deref) = count_derefs(self.cx, expr);
            self.n_derefs = min(self.n_derefs, n_derefs);
            self.explicit_deref |= explicit_deref;
        }
        walk_expr(self, expr);
    }
}

fn count_derefs<'tcx>(cx: &LateContext<'tcx>, mut expr: &Expr<'tcx>) -> (usize, bool) {
    let mut n_derefs = 0;
    let mut explicit_deref = false;
    let mut parent_iter = cx.tcx.hir_parent_iter(expr.hir_id);
    loop {
        let adjustments = cx.typeck_results().expr_adjustments(expr);
        // `adjusted_for_deref` is meant to catch cases like the following:
        // [Deref(None) -> X, Borrow(Ref(ReErased, Not)) -> &X]
        let mut adjusted_for_deref = false;
        for adjustment in adjustments {
            match adjustment.kind {
                Adjust::Deref(_) => {
                    if is_copy(cx, adjustment.target) {
                        n_derefs += 1;
                        adjusted_for_deref = true;
                    } else {
                        return (n_derefs, explicit_deref);
                    }
                }
                Adjust::Borrow(_) => {
                    if adjusted_for_deref {
                        n_derefs -= 1;
                    }
                    return (n_derefs, explicit_deref);
                }
                _ => {
                    return (n_derefs, explicit_deref);
                }
            }
        }
        if let Some((_, Node::Expr(parent_expr))) = parent_iter.next()
            && matches!(parent_expr.kind, ExprKind::Unary(UnOp::Deref, _))
            && let parent_expr_ty = cx.typeck_results().expr_ty(parent_expr)
            && is_copy(cx, parent_expr_ty)
        {
            n_derefs += 1;
            explicit_deref = true;
            expr = parent_expr;
        } else {
            return (n_derefs, explicit_deref);
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[test]
fn ui_no_explicit_deref_check() {
    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui_no_explicit_deref_check")
        .dylint_toml("suboptimal_pattern.explicit_deref_check = false")
        .run();
}

#[test]
fn ui_main_rs_are_equal() {
    let ui_main_rs = std::fs::read_to_string("ui/main.rs").unwrap();

    let ui_no_explicit_deref_check_main_rs =
        std::fs::read_to_string("ui_no_explicit_deref_check/main.rs").unwrap();

    assert_eq!(ui_main_rs, ui_no_explicit_deref_check_main_rs);
}
