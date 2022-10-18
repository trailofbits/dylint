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
    ty::{is_copy, peel_mid_ty_refs},
};
use dylint_internal::env::enabled;
use if_chain::if_chain;
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_hir::{
    intravisit::{walk_expr, FnKind, Visitor},
    BindingAnnotation, Body, ByRef, Expr, ExprKind, FnDecl, HirId, Node, Pat, PatKind, UnOp,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, adjustment::Adjust};
use rustc_span::Span;
use std::{cmp::min, fmt::Write};

dylint_linting::declare_late_lint! {
    /// **What it does:** Checks for patterns that could perform additional destructuring.
    ///
    /// **Why is this bad?** The use of destructuring patterns in closure parameters (for example)
    /// often leads to more concise closure bodies. Beyond that, the benefits of this lint are
    /// similar to those of [pattern-type-mismatch].
    ///
    /// **Known problems:**
    /// * Currently only checks closure parameters (not, e.g., match patterns).
    /// * Currently only suggests destructuring references and tuples (not, e.g., arrays or
    ///   structs).
    /// * For the lint to suggest destructuring a reference, the idents involved must not use `ref`
    ///   annotations.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// let xs = [0, 1, 2];
    /// let ys = xs.iter().map(|x| *x == 0).collect::<Vec<_>>();
    /// ```
    /// Use instead:
    /// ```rust
    /// let xs = [0, 1, 2];
    /// let ys = xs.iter().map(|&x| x == 0).collect::<Vec<_>>();
    /// ```
    ///
    /// **Options:**
    /// `SUBOPTIMAL_PATTERN_NO_EXPLICIT_DEREF_CHECK`: By default, `suboptimal_pattern` will not
    /// suggest to destructure a reference unless it would eliminate at least one explicit
    /// dereference. Setting this environment variable to anything other than `0` disables this
    /// check.
    ///
    /// [pattern-type-mismatch]: https://rust-lang.github.io/rust-clippy/master/#pattern_type_mismatch
    pub SUBOPTIMAL_PATTERN,
    Warn,
    "patterns that could perform additional destructuring"
}

const SUBOPTIMAL_PATTERN_NO_EXPLICIT_DEREF_CHECK: &str =
    "SUBOPTIMAL_PATTERN_NO_EXPLICIT_DEREF_CHECK";

impl<'tcx> LateLintPass<'tcx> for SuboptimalPattern {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        fn_kind: FnKind<'tcx>,
        _: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _: HirId,
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
                let pat_ty = cx.typeck_results().node_type(pat.hir_id);
                let (referent_ty, n_refs) = peel_mid_ty_refs(pat_ty);

                if_chain! {
                    if let ty::Tuple(tys) = referent_ty.kind();
                    if let PatKind::Binding(BindingAnnotation(ByRef::No, _), hir_id, ident, None) =
                        pat.kind;
                    if let Some(projections) = exclusively_projected(cx.tcx, hir_id, &body.value);
                    then {
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

                if_chain! {
                    if !matches!(pat.kind, PatKind::Wild);
                    if let Some(hir_ids) = collect_non_ref_idents(pat);
                    if let Some(n_derefs) = exclusively_dereferenced(cx, hir_ids, &body.value);
                    if n_derefs > 0;
                    then {
                        let snippet = snippet(cx, pat.span, "_");
                        let pattern = format!("{:&>width$}{}", "", snippet, width = n_derefs);
                        span_lint_and_sugg(
                            cx,
                            SUBOPTIMAL_PATTERN,
                            pat.span,
                            &format!(
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
                }

                true
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
            let parent_hir_id = self.tcx.hir().get_parent_node(expr.hir_id);
            let node = self.tcx.hir().get(parent_hir_id);
            if_chain! {
                if let Node::Expr(Expr {
                    kind: ExprKind::Field(_, ident),
                    ..
                }) = node;
                if let Ok(projection) = ident.name.as_str().parse::<usize>();
                then {
                    self.projections
                        .as_mut()
                        .map(|projections| projections.insert(projection));
                } else {
                    self.projections = None;
                }
            }
        }
        walk_expr(self, expr);
    }
}

fn build_tuple_pattern(ident: &str, projections: &FxHashSet<usize>, size: usize) -> String {
    let mut buf = "(".to_owned();
    for i in 0..size {
        if projections.contains(&i) {
            write!(buf, "{}_{}", ident, i).unwrap();
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

fn collect_non_ref_idents(pat: &Pat<'_>) -> Option<FxHashSet<HirId>> {
    let mut hir_ids = Some(FxHashSet::default());
    pat.walk(|pat| {
        if let PatKind::Binding(annotation, _, _, _) = pat.kind {
            match annotation {
                BindingAnnotation(ByRef::No, _) => {
                    hir_ids.as_mut().map(|hir_ids| hir_ids.insert(pat.hir_id));
                }
                BindingAnnotation(ByRef::Yes, _) => {
                    hir_ids = None;
                }
            }
        }
        hir_ids.is_some()
    });
    hir_ids
}

fn exclusively_dereferenced<'tcx>(
    cx: &LateContext<'tcx>,
    hir_ids: FxHashSet<HirId>,
    expr: &'tcx Expr<'tcx>,
) -> Option<usize> {
    let mut visitor = DereferenceVisitor {
        cx,
        hir_ids,
        n_derefs: usize::MAX,
        explicit_deref: enabled(SUBOPTIMAL_PATTERN_NO_EXPLICIT_DEREF_CHECK),
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

impl<'cx, 'tcx> Visitor<'tcx> for DereferenceVisitor<'cx, 'tcx> {
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
    let mut parent_iter = cx.tcx.hir().parent_iter(expr.hir_id);
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
        if_chain! {
            if let Some((_, Node::Expr(parent_expr))) = parent_iter.next();
            if matches!(parent_expr.kind, ExprKind::Unary(UnOp::Deref, _));
            let parent_expr_ty = cx.typeck_results().expr_ty(parent_expr);
            if is_copy(cx, parent_expr_ty);
            then {
                n_derefs += 1;
                explicit_deref = true;
                expr = parent_expr;
            } else {
                return (n_derefs, explicit_deref);
            }
        }
    }
}

#[cfg_attr(
    dylint_lib = "non_thread_safe_call_in_test",
    allow(non_thread_safe_call_in_test)
)]
#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );

    // smoelius: See comment in `redundant_reference` regarding `compiletest` and environment
    // variables.
    std::env::set_var(SUBOPTIMAL_PATTERN_NO_EXPLICIT_DEREF_CHECK, "1");

    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui_no_explicit_deref_check"),
    );
}

#[test]
fn ui_main_rs_are_equal() {
    let ui_main_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("ui")
            .join("main.rs"),
    )
    .unwrap();

    let ui_no_explicit_deref_check_main_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("ui_no_explicit_deref_check")
            .join("main.rs"),
    )
    .unwrap();

    assert_eq!(ui_main_rs, ui_no_explicit_deref_check_main_rs);
}
