use crate::type_name;
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::{CRATE_HIR_ID, HirId, intravisit::Visitor};
use rustc_lint::LateContext;
use rustc_middle::ty::TyCtxt;
use std::{cell::OnceCell, collections::BTreeMap};

thread_local! {
    // smoelius: The `HirId`s in each `Vec` are ordered "more ancestral" to "less ancestral". In
    // particular, if x is an ancestor of y, x should appear before y in the `Vec`.
    // smoelius: Also, the following spans are filtered out:
    //
    // - "no-location" spans, i.e., spans with no associated source file
    // - "from expansion" spans, i.e., spans produced by macros
    // - spans that do not hold all of their children
    //
    // For the last bullet, if a child is determined to be larger than its parent, the parent is
    // discarded before the child is inserted.
    static SPAN_HIR_ID_MAP: OnceCell<BTreeMap<rustc_span::Span, Vec<HirId>>> = const { OnceCell::new() };
}

pub(crate) fn hir_ids_from_span<T>(cx: &LateContext, span: rustc_span::Span) -> Vec<HirId> {
    __hir_ids_from_span_untyped(cx, span)
        .iter()
        .filter(|&&hir_id| {
            let node = cx.tcx.hir_node(hir_id);
            type_name(node) == Some(std::any::type_name::<T>())
        })
        .copied()
        .collect()
}

pub fn __hir_ids_from_span_untyped(cx: &LateContext, span: rustc_span::Span) -> Vec<HirId> {
    SPAN_HIR_ID_MAP.with(|map| {
        let map = map.get_or_init(|| init(cx));

        if let Some(hir_ids) = map.get(&span) {
            hir_ids.clone()
        } else {
            Vec::default()
        }
    })
}

// https://doc.rust-lang.org/beta/nightly-rustc/rustc_hir/intravisit/index.html
fn init(cx: &LateContext) -> BTreeMap<rustc_span::Span, Vec<HirId>> {
    let mut visitor = SpanHirIdVisitor {
        tcx: cx.tcx,
        map: BTreeMap::default(),
        discarded: FxHashSet::default(),
    };
    #[allow(clippy::disallowed_methods)]
    let crate_span = cx.tcx.hir_span(CRATE_HIR_ID);
    visitor.visit_mod(cx.tcx.hir_root_module(), crate_span, CRATE_HIR_ID);
    visitor.map
}

struct SpanHirIdVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    map: BTreeMap<rustc_span::Span, Vec<HirId>>,
    discarded: FxHashSet<HirId>,
}

impl<'tcx> Visitor<'tcx> for SpanHirIdVisitor<'tcx> {
    type NestedFilter = rustc_middle::hir::nested_filter::All;

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.tcx
    }

    fn visit_id(&mut self, hir_id: HirId) {
        let Some(span) = root_span_with_location(self.tcx, hir_id) else {
            return;
        };
        for parent_id in self.tcx.hir_parent_id_iter(hir_id) {
            let Some(parent_span) = root_span_with_location(self.tcx, parent_id) else {
                continue;
            };
            // smoelius: We want to insert the span into the tree, but we also want to preserve the
            // invariant that each span is contained by its ancestors in the branch. So, search
            // until you find an ancestor that contains the span and then stop. The span will be
            // contained in that ancestor, and that ancestor will be contained in all of its
            // ancestors inductively.
            if parent_span.contains(span) {
                // smoelius: Ensure the parent span is in the map.
                let parent_vec = self.map.entry(parent_span).or_default();
                if parent_vec.last() != Some(&parent_id) && !self.discarded.contains(&parent_id) {
                    parent_vec.push(parent_id);
                }
                break;
            }
            // smoelius: The parent span does not contain span. Remove the parent span and keep
            // searching.
            let Some(parent_vec) = self.map.get_mut(&parent_span) else {
                continue;
            };
            // smoelius: The parent span could have already been discarded.
            if parent_vec.last() == Some(&parent_id) {
                let _: Option<HirId> = parent_vec.pop();
                self.discarded.insert(parent_id);
            }
        }

        self.map.entry(span).or_default().push(hir_id);
    }
}

fn root_span_with_location(tcx: TyCtxt, hir_id: HirId) -> Option<rustc_span::Span> {
    #[allow(clippy::disallowed_methods)]
    let span = tcx.hir_span(hir_id);

    if is_no_location_span(tcx, span) || span.from_expansion() {
        return None;
    }

    Some(span)
}

fn is_no_location_span(tcx: TyCtxt, span: rustc_span::Span) -> bool {
    let (source_file, _lo_line, _lo_col, _hi_line, _hi_col) =
        tcx.sess.source_map().span_to_location_info(span);

    source_file.is_none()
}
