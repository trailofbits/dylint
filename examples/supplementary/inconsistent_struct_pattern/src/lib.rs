#![feature(rustc_private)]
#![feature(let_chains)]
#![warn(unused_extern_crates)]

extern crate rustc_data_structures;
extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{
    def::{DefKind, Res},
    Pat, PatField, PatKind, Path, QPath,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Symbol;

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Checks for struct patterns whose fields whose fields do not match their declared order.
    ///
    /// ### Why is this bad?
    /// It can be harder to spot mistakes in inconsistent code.
    ///
    /// ### Example
    /// ```rust
    /// struct Struct {
    ///     a: bool,
    ///     b: bool,
    /// };
    /// let strukt = Struct { a: false, b: true };
    /// let Struct { b, a } = strukt;
    /// ```
    /// Use instead:
    /// ```rust
    /// struct Struct {
    ///     a: bool,
    ///     b: bool,
    /// };
    /// let strukt = Struct { a: false, b: true };
    /// let Struct { a, b } = strukt;
    /// ```
    pub INCONSISTENT_STRUCT_PATTERN,
    Warn,
    "struct patterns whose fields do not match their declared order"
}

impl<'tcx> LateLintPass<'tcx> for InconsistentStructPattern {
    fn check_pat(&mut self, cx: &LateContext<'tcx>, pat: &'tcx Pat<'tcx>) {
        let PatKind::Struct(
            QPath::Resolved(
                _,
                Path {
                    res: Res::Def(DefKind::Struct, def_id),
                    ..
                },
            ),
            fields,
            _,
        ) = pat.kind
        else {
            return;
        };

        let adt_def = cx.tcx.adt_def(def_id);
        let variant_def = adt_def.variants().iter().next().unwrap();

        let mut def_order_map = FxHashMap::default();
        for (idx, field) in variant_def.fields.iter().enumerate() {
            def_order_map.insert(field.name, idx);
        }

        if is_consistent_order(fields, &def_order_map) {
            return;
        }

        span_lint(
            cx,
            INCONSISTENT_STRUCT_PATTERN,
            pat.span,
            "struct pattern field order is inconsistent with struct definition field order",
        );
    }
}

// smoelius: `is_consistent_order` is based on:
// https://github.com/rust-lang/rust-clippy/blob/35e8be7407198565c434b69c5b9f85c71f156539/clippy_lints/src/inconsistent_struct_constructor.rs#L120-L133

// Check whether the order of the fields in the constructor is consistent with the order in the
// definition.
fn is_consistent_order<'tcx>(
    fields: &'tcx [PatField<'tcx>],
    def_order_map: &FxHashMap<Symbol, usize>,
) -> bool {
    let mut cur_idx = usize::MIN;
    for f in fields {
        let next_idx = def_order_map[&f.ident.name];
        if cur_idx > next_idx {
            return false;
        }
        cur_idx = next_idx;
    }

    true
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
