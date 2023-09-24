use marker_api::ast::{
    generic::{
        Lifetime, LifetimeKind, SemBindingArg, SemConstArg, SemGenericArgKind, SemGenericArgs,
        SemTraitBound, SynBindingArg, SynConstArg, SynConstParam, SynGenericArgKind,
        SynGenericArgs, SynGenericParamKind, SynGenericParams, SynLifetimeArg, SynLifetimeClause,
        SynLifetimeParam, SynTraitBound, SynTyArg, SynTyClause, SynTyParam, SynTyParamBound,
        SynWhereClauseKind,
    },
    ConstValue, TraitRef,
};
use rustc_hir as hir;
use rustc_middle as mid;

use super::MarkerConverterInner;

impl<'ast, 'tcx> MarkerConverterInner<'ast, 'tcx> {
    #[must_use]
    pub fn to_lifetime(&self, rust_lt: &hir::Lifetime) -> Option<Lifetime<'ast>> {
        let kind = match rust_lt.res {
            hir::LifetimeName::Param(_) if rust_lt.is_anonymous() => return None,
            hir::LifetimeName::Param(local_id) => LifetimeKind::Label(
                self.to_symbol_id(rust_lt.ident.name),
                self.to_generic_id(local_id),
            ),
            hir::LifetimeName::ImplicitObjectLifetimeDefault => return None,
            hir::LifetimeName::Infer => LifetimeKind::Infer,
            hir::LifetimeName::Static => LifetimeKind::Static,
            hir::LifetimeName::Error => unreachable!("would have triggered a rustc error"),
        };

        Some(Lifetime::new(
            Some(self.to_span_id(rust_lt.ident.span)),
            kind,
        ))
    }
}

impl<'ast, 'tcx> MarkerConverterInner<'ast, 'tcx> {
    #[must_use]
    pub fn to_sem_generic_args(&self, args: &[mid::ty::GenericArg<'tcx>]) -> SemGenericArgs<'ast> {
        let args: Vec<_> = args
            .iter()
            .filter_map(|arg| self.to_sem_generic_arg_kind(*arg))
            .collect();

        SemGenericArgs::new(self.alloc_slice(args))
    }

    #[must_use]
    fn to_sem_generic_arg_kind(
        &self,
        arg: mid::ty::GenericArg<'tcx>,
    ) -> Option<SemGenericArgKind<'ast>> {
        match &arg.unpack() {
            mid::ty::GenericArgKind::Lifetime(_) => None,
            mid::ty::GenericArgKind::Type(ty) => Some(SemGenericArgKind::Ty(self.to_sem_ty(*ty))),
            mid::ty::GenericArgKind::Const(_) => Some(SemGenericArgKind::Const(
                self.alloc(SemConstArg::new(ConstValue::new())),
            )),
        }
    }

    pub fn to_sem_trait_bounds(
        &self,
        bounds: &mid::ty::List<mid::ty::PolyExistentialPredicate<'tcx>>,
    ) -> &'ast [SemTraitBound<'ast>] {
        let mut marker_bounds = vec![];

        // Understanding this representation, was a journey of at least 1.5 liters
        // of tea, way too many print statements and complaining to a friend of mine.
        //
        // Here is the basic breakdown:
        // * Due to [`E0225`] these bounds are currently restricted to one *main* trait. Any other
        //   traits have to be auto traits.
        // * Simple generic args, like the `u32` in `Trait<u32>`, are stored in the `substs` of the
        //   trait.
        // * Named type parameters, like `Item = u32` in `dyn Iterator<Item = u32>`, are stored as
        //   `ExistentialPredicate::Projection` in the list of bindings. These parameters now need
        //   to be *reattached* to the `SemGenericArgs` of the *main* trait, to work with markers
        //   representation.
        //
        // [`E0225`]: https://doc.rust-lang.org/stable/error_codes/E0225.html
        if let Some(main) = bounds.principal() {
            let main = main.skip_binder();

            let mut generics: Vec<_> = main
                .substs
                .iter()
                .filter_map(|arg| self.to_sem_generic_arg_kind(arg))
                .collect();

            bounds.projection_bounds().for_each(|binding| {
                match binding.skip_binder().term.unpack() {
                    mid::ty::TermKind::Ty(ty) => {
                        generics.push(SemGenericArgKind::Binding(self.alloc(SemBindingArg::new(
                            self.to_item_id(binding.item_def_id()),
                            self.to_sem_ty(ty),
                        ))))
                    }
                    mid::ty::TermKind::Const(_) => todo!(),
                }
            });

            marker_bounds.push(SemTraitBound::new(
                false,
                self.to_ty_def_id(main.def_id),
                SemGenericArgs::new(self.alloc_slice(generics)),
            ));
        }

        bounds
            .auto_traits()
            .map(|auto_trait_id| {
                SemTraitBound::new(
                    false,
                    self.to_ty_def_id(auto_trait_id),
                    self.to_sem_generic_args(&[]),
                )
            })
            .collect_into(&mut marker_bounds);

        self.alloc_slice(marker_bounds)
    }
}

impl<'ast, 'tcx> MarkerConverterInner<'ast, 'tcx> {
    pub fn to_syn_generic_args_from_path(
        &self,
        rust_path: &rustc_hir::Path<'tcx>,
    ) -> SynGenericArgs<'ast> {
        self.to_syn_generic_args(rust_path.segments.last().and_then(|s| s.args))
    }

    #[must_use]
    pub fn to_syn_generic_args(
        &self,
        rustc_args: Option<&hir::GenericArgs<'tcx>>,
    ) -> SynGenericArgs<'ast> {
        let Some(rustc_args) = rustc_args else {
            return SynGenericArgs::new(&[]);
        };

        let mut args: Vec<_> = rustc_args
            .args
            .iter()
            .filter(|rustc_arg| !rustc_arg.is_synthetic())
            .filter_map(|rustc_arg| match rustc_arg {
                rustc_hir::GenericArg::Lifetime(rust_lt) => {
                    self.to_lifetime(rust_lt).map(|lifetime| {
                        SynGenericArgKind::Lifetime(self.alloc(SynLifetimeArg::new(lifetime)))
                    })
                }
                rustc_hir::GenericArg::Type(r_ty) => Some(SynGenericArgKind::Ty(
                    self.alloc(SynTyArg::new(self.to_syn_ty(r_ty))),
                )),
                rustc_hir::GenericArg::Const(arg) => Some(SynGenericArgKind::Const(self.alloc(
                    SynConstArg::new(self.to_span_id(arg.span), self.to_const_expr(arg.value)),
                ))),
                rustc_hir::GenericArg::Infer(_) => todo!(),
            })
            .collect();
        args.extend(
            rustc_args
                .bindings
                .iter()
                .map(|binding| match &binding.kind {
                    rustc_hir::TypeBindingKind::Equality { term } => match term {
                        rustc_hir::Term::Ty(rustc_ty) => SynGenericArgKind::Binding(self.alloc({
                            SynBindingArg::new(
                                self.to_span_id(binding.span),
                                self.to_symbol_id(binding.ident.name),
                                self.to_syn_ty(rustc_ty),
                            )
                        })),
                        rustc_hir::Term::Const(_) => todo!(),
                    },
                    rustc_hir::TypeBindingKind::Constraint { .. } => todo!(),
                }),
        );
        SynGenericArgs::new(self.alloc_slice(args))
    }

    pub fn to_syn_generic_params(
        &self,
        rustc_generics: &hir::Generics<'tcx>,
    ) -> SynGenericParams<'ast> {
        let clauses: Vec<_> = rustc_generics
            .predicates
            .iter()
            .filter_map(|predicate| {
                match predicate {
                    hir::WherePredicate::BoundPredicate(ty_bound) => {
                        // FIXME Add span to API clause:
                        // let span = to_api_span_id(ty_bound.span);
                        let params = SynGenericParams::new(
                            self.to_syn_generic_param_kinds(ty_bound.bound_generic_params),
                            &[],
                        );
                        let ty = self.to_syn_ty(ty_bound.bounded_ty);
                        Some(SynWhereClauseKind::Ty(self.alloc({
                            SynTyClause::new(
                                Some(params),
                                ty,
                                self.to_syn_ty_param_bound(predicate.bounds()),
                            )
                        })))
                    }
                    hir::WherePredicate::RegionPredicate(lifetime_bound) => {
                        self.to_lifetime(lifetime_bound.lifetime).map(|lifetime| {
                            SynWhereClauseKind::Lifetime(self.alloc({
                                let bounds: Vec<_> = lifetime_bound
                                    .bounds
                                    .iter()
                                    .filter_map(|bound| match bound {
                                        hir::GenericBound::Outlives(lifetime) => {
                                            self.to_lifetime(lifetime)
                                        }
                                        _ => {
                                            unreachable!("lifetimes can only be bound by lifetimes")
                                        }
                                    })
                                    .collect();
                                let bounds = if bounds.is_empty() {
                                    self.alloc_slice(bounds)
                                } else {
                                    &[]
                                };
                                SynLifetimeClause::new(lifetime, bounds)
                            }))
                        })
                    }
                    hir::WherePredicate::EqPredicate(_) => {
                        unreachable!("the documentation states, that this is unsupported")
                    }
                }
            })
            .collect();
        let clauses = self.alloc_slice(clauses);

        SynGenericParams::new(
            self.to_syn_generic_param_kinds(rustc_generics.params),
            clauses,
        )
    }

    fn to_syn_generic_param_kinds(
        &self,
        params: &[hir::GenericParam<'tcx>],
    ) -> &'ast [SynGenericParamKind<'ast>] {
        if params.is_empty() {
            return &[];
        }

        let params: Vec<_> = params
            .iter()
            .filter_map(|rustc_param| {
                let name = match rustc_param.name {
                    hir::ParamName::Plain(ident) => self.to_symbol_id(ident.name),
                    _ => return None,
                };
                let id = self.to_generic_id(rustc_param.def_id);
                let span = self.to_span_id(rustc_param.span);
                match rustc_param.kind {
                    hir::GenericParamKind::Lifetime {
                        kind: hir::LifetimeParamKind::Explicit,
                    } => Some(SynGenericParamKind::Lifetime(
                        self.alloc(SynLifetimeParam::new(id, name, Some(span))),
                    )),
                    hir::GenericParamKind::Type {
                        synthetic: false, ..
                    } => Some(SynGenericParamKind::Ty(self.alloc(SynTyParam::new(
                        Some(span),
                        name,
                        id,
                    )))),
                    hir::GenericParamKind::Const { ty, default } => {
                        Some(SynGenericParamKind::Const(self.alloc(SynConstParam::new(
                            id,
                            name,
                            self.to_syn_ty(ty),
                            default.map(|anon| self.to_const_expr(anon)),
                            span,
                        ))))
                    }
                    _ => None,
                }
            })
            .collect();

        self.alloc_slice(params)
    }

    #[must_use]
    pub fn to_syn_ty_param_bound(
        &self,
        bounds: &[hir::GenericBound<'tcx>],
    ) -> &'ast [SynTyParamBound<'ast>] {
        if bounds.is_empty() {
            return &[];
        }

        let bounds: Vec<_> =
            bounds
                .iter()
                .filter_map(|bound| match bound {
                    hir::GenericBound::Trait(trait_ref, modifier) => {
                        Some(SynTyParamBound::TraitBound(self.alloc(SynTraitBound::new(
                            !matches!(modifier, hir::TraitBoundModifier::None),
                            self.to_trait_ref(&trait_ref.trait_ref),
                            self.to_span_id(bound.span()),
                        ))))
                    }
                    hir::GenericBound::LangItemTrait(lang_item, span, _, rustc_args) => {
                        Some(SynTyParamBound::TraitBound(self.alloc(SynTraitBound::new(
                            false,
                            TraitRef::new(
                                self.to_item_id(
                                    self.rustc_cx.get_lang_items(()).get(*lang_item).expect(
                                        "the lang item is used and should therefore be loaded",
                                    ),
                                ),
                                self.to_syn_generic_args(Some(rustc_args)),
                            ),
                            self.to_span_id(*span),
                        ))))
                    }
                    hir::GenericBound::Outlives(rust_lt) => self
                        .to_lifetime(rust_lt)
                        .map(|api_lt| SynTyParamBound::Lifetime(self.alloc(api_lt))),
                })
                .collect();

        self.alloc_slice(bounds)
    }

    pub fn to_syn_ty_param_bound_from_hir(
        &self,
        rust_bounds: &[rustc_hir::PolyTraitRef<'tcx>],
        rust_lt: &rustc_hir::Lifetime,
    ) -> &'ast [SynTyParamBound<'ast>] {
        let traits = rust_bounds.iter().map(|rust_trait_ref| {
            SynTyParamBound::TraitBound(self.storage.alloc(SynTraitBound::new(
                false,
                self.to_trait_ref(&rust_trait_ref.trait_ref),
                self.to_span_id(rust_trait_ref.span),
            )))
        });

        if let Some(lt) = self.to_lifetime(rust_lt) {
            // alloc_slice_iter requires a const size, which is not possible otherwise
            let mut bounds: Vec<_> = traits.collect();
            bounds.push(SynTyParamBound::Lifetime(self.alloc(lt)));
            self.alloc_slice(bounds)
        } else {
            self.alloc_slice(traits)
        }
    }
}
