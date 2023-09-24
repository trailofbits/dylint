use marker_api::{
    ast::{
        expr::ExprKind,
        pat::{
            CommonPatData, IdentPat, OrPat, PatKind, PathPat, RangePat, RefPat, RestPat, SlicePat,
            StructFieldPat, StructPat, TuplePat, UnstablePat, WildcardPat,
        },
    },
    CtorBlocker,
};
use rustc_hash::FxHashMap;
use rustc_hir as hir;

use super::MarkerConverterInner;

thread_local! {
    static DEFAULT_LHS_MAP: FxHashMap<hir::HirId, ExprKind<'static>> = FxHashMap::default();
}

impl<'ast, 'tcx> MarkerConverterInner<'ast, 'tcx> {
    #[must_use]
    pub fn to_pat(&self, pat: &hir::Pat<'tcx>) -> PatKind<'ast> {
        DEFAULT_LHS_MAP.with(|map| self.to_pat_with_hls(pat, map))
    }

    #[must_use]
    pub fn to_pat_with_hls(
        &self,
        pat: &hir::Pat<'tcx>,
        lhs_map: &FxHashMap<hir::HirId, ExprKind<'ast>>,
    ) -> PatKind<'ast> {
        // Here we don't need to take special care for caching, as marker patterns
        // don't have IDs and can't be requested individually. Instead patterns are
        // stored as part of their parent expressions or items. Not needing to deal
        // with caching makes this implementation simpler.
        let data = CommonPatData::new(self.to_span_id(pat.span));

        match &pat.kind {
            hir::PatKind::Wild => PatKind::Wildcard(self.alloc(WildcardPat::new(data))),
            hir::PatKind::Binding(hir::BindingAnnotation(by_ref, mutab), id, ident, pat) => {
                let lhs = lhs_map.get(id);
                #[allow(clippy::unnecessary_unwrap, reason = "if let sadly breaks rustfmt")]
                if pat.is_none() && matches!(mutab, rustc_ast::Mutability::Not) && lhs.is_some() {
                    PatKind::Place(*lhs.unwrap(), CtorBlocker::new())
                } else {
                    PatKind::Ident(self.alloc({
                        IdentPat::new(
                            data,
                            self.to_symbol_id(ident.name),
                            self.to_var_id(*id),
                            self.to_mutability(*mutab),
                            matches!(by_ref, hir::ByRef::Yes),
                            pat.map(|rustc_pat| self.to_pat_with_hls(rustc_pat, lhs_map)),
                        )
                    }))
                }
            }
            hir::PatKind::Struct(qpath, fields, has_rest) => {
                let api_fields = self.alloc_slice(fields.iter().map(|field| {
                    StructFieldPat::new(
                        self.to_span_id(field.span),
                        self.to_symbol_id(field.ident.name),
                        self.to_pat_with_hls(field.pat, lhs_map),
                    )
                }));
                PatKind::Struct(self.alloc(StructPat::new(
                    data,
                    self.to_qpath_from_pat(qpath, pat),
                    api_fields,
                    *has_rest,
                )))
            }
            hir::PatKind::TupleStruct(qpath, pats, dotdot) => {
                let ddpos = dotdot.as_opt_usize();
                let offset_pos = ddpos.unwrap_or(usize::MAX);
                let api_fields =
                    self.alloc_slice(pats.iter().enumerate().map(|(mut index, pat)| {
                        if index >= offset_pos {
                            index += offset_pos;
                        }
                        StructFieldPat::new(
                            self.to_span_id(pat.span),
                            self.to_symbol_id_for_num(
                                u32::try_from(index).expect("a index over 2^32 is unexpected"),
                            ),
                            self.to_pat_with_hls(pat, lhs_map),
                        )
                    }));
                PatKind::Struct(self.alloc(StructPat::new(
                    data,
                    self.to_qpath_from_pat(qpath, pat),
                    api_fields,
                    ddpos.is_some(),
                )))
            }
            hir::PatKind::Or(pats) => PatKind::Or(self.alloc(OrPat::new(
                data,
                self.alloc_slice(pats.iter().map(|rpat| self.to_pat_with_hls(rpat, lhs_map))),
            ))),
            hir::PatKind::Tuple(pats, dotdot) => {
                let pats = if let Some(rest_pos) = dotdot.as_opt_usize() {
                    let (start, end) = pats.split_at(rest_pos);
                    // This is a dummy span, it's dirty, but at least works for the mean time :)
                    self.chain_pats(start, self.new_rest_pat(rustc_span::DUMMY_SP), end, lhs_map)
                } else {
                    self.alloc_slice(pats.iter().map(|pat| self.to_pat_with_hls(pat, lhs_map)))
                };
                PatKind::Tuple(self.alloc(TuplePat::new(data, pats)))
            }
            hir::PatKind::Box(_) => PatKind::Unstable(self.alloc(UnstablePat::new(data))),
            hir::PatKind::Ref(pat, muta) => PatKind::Ref(self.alloc(RefPat::new(
                data,
                self.to_pat_with_hls(pat, lhs_map),
                self.to_mutability(*muta),
            ))),
            hir::PatKind::Slice(start, wild, end) => {
                let elements = if let Some(wild) = wild {
                    self.chain_pats(start, self.new_rest_pat(wild.span), end, lhs_map)
                } else {
                    assert!(end.is_empty());
                    self.alloc_slice(start.iter().map(|pat| self.to_pat_with_hls(pat, lhs_map)))
                };
                PatKind::Slice(self.alloc(SlicePat::new(data, elements)))
            }
            hir::PatKind::Path(path) => {
                PatKind::Path(self.alloc(PathPat::new(data, self.to_qpath_from_pat(path, pat))))
            }
            hir::PatKind::Lit(lit) => {
                let expr = self.to_expr(lit);
                let lit_expr = expr
                    .try_into()
                    .unwrap_or_else(|_| panic!("this should be a literal expression {lit:#?}"));
                PatKind::Lit(lit_expr, CtorBlocker::new())
            }
            hir::PatKind::Range(start, end, kind) => PatKind::Range(self.alloc(RangePat::new(
                data,
                start.map(|expr| self.to_expr(expr)),
                end.map(|expr| self.to_expr(expr)),
                matches!(kind, hir::RangeEnd::Included),
            ))),
        }
    }

    fn chain_pats(
        &self,
        start: &[hir::Pat<'tcx>],
        ast_wild: PatKind<'ast>,
        end: &[hir::Pat<'tcx>],
        lhs_map: &FxHashMap<hir::HirId, ExprKind<'ast>>,
    ) -> &'ast [PatKind<'ast>] {
        let start = start.iter().map(|pat| self.to_pat_with_hls(pat, lhs_map));
        let middle = std::iter::once(ast_wild);
        let end = end.iter().map(|pat| self.to_pat_with_hls(pat, lhs_map));
        let api_pats: Vec<_> = start.chain(middle).chain(end).collect();
        self.alloc_slice(api_pats)
    }

    fn new_rest_pat(&self, span: rustc_span::Span) -> PatKind<'ast> {
        let data = CommonPatData::new(self.to_span_id(span));
        PatKind::Rest(self.alloc(RestPat::new(data)))
    }
}
