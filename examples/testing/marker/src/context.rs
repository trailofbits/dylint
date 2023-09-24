use std::cell::{OnceCell, RefCell};

use marker_adapter::context::{DriverContext, DriverContextWrapper};
use marker_api::{
    ast::{
        item::{Body, ItemKind},
        BodyId, ExprId, ItemId, Span, SpanId, SymbolId, TyDefId,
    },
    context::AstContext,
    diagnostic::{Diagnostic, EmissionNode},
    lint::{Level, Lint},
};
use rustc_hash::FxHashMap;
use rustc_hir as hir;
use rustc_lint::LintStore;
use rustc_middle::ty::TyCtxt;

use crate::conversion::{marker::MarkerConverter, rustc::RustcConverter};

use self::storage::Storage;

pub mod storage;

/// This is the central context for the rustc driver and the struct providing the callback
/// implementation for [`AstContext`](`marker_api::context::AstContext`).
///
/// The struct intentionally only stores the [`TyCtxt`] and [`LintStore`] and not a
/// [`LateContext`](`rustc_lint::LateContext`) as the late context operates on the assumption that
/// every AST node is only checked in the specific `check_` function. This will in contrast convert
/// the entire crate at once and might also jump around inside the AST if a lint crate requests
/// that. This also has the added benefit that we can use the `'tcx` lifetime for them.
pub struct RustcContext<'ast, 'tcx> {
    pub rustc_cx: TyCtxt<'tcx>,
    pub lint_store: &'tcx LintStore,

    pub storage: &'ast Storage<'ast>,
    pub marker_converter: MarkerConverter<'ast, 'tcx>,
    pub rustc_converter: RustcConverter<'ast, 'tcx>,

    /// This is the [`AstContext`] wrapping callbacks to this instance of the
    /// [`RustcContext`]. The once cell will be set immediately after the creation
    /// which makes it safe to access afterwards.
    ast_cx: OnceCell<&'ast AstContext<'ast>>,
    resolved_ty_ids: RefCell<FxHashMap<&'ast str, &'ast [TyDefId]>>,
}

impl<'ast, 'tcx> RustcContext<'ast, 'tcx> {
    pub fn new(
        rustc_cx: TyCtxt<'tcx>,
        lint_store: &'tcx LintStore,
        storage: &'ast Storage<'ast>,
    ) -> &'ast Self {
        // Create context
        let driver_cx = storage.alloc(Self {
            rustc_cx,
            lint_store,
            storage,
            marker_converter: MarkerConverter::new(rustc_cx, storage),
            rustc_converter: RustcConverter::new(rustc_cx),
            ast_cx: OnceCell::new(),
            resolved_ty_ids: RefCell::default(),
        });

        // Create and link `AstContext`
        let callbacks_wrapper = storage.alloc(DriverContextWrapper::new(driver_cx));
        let callbacks = storage.alloc(callbacks_wrapper.create_driver_callback());
        let ast_cx = storage.alloc(AstContext::new(callbacks));
        driver_cx.ast_cx.set(ast_cx).unwrap();

        driver_cx
    }

    pub fn ast_cx(&self) -> &'ast AstContext<'ast> {
        // The `OnceCell` is filled in the new function and can never be not set.
        self.ast_cx.get().unwrap()
    }
}

impl<'ast, 'tcx: 'ast> DriverContext<'ast> for RustcContext<'ast, 'tcx> {
    fn lint_level_at(&'ast self, api_lint: &'static Lint, node: EmissionNode) -> Level {
        if let Some(id) = self.rustc_converter.try_to_hir_id_from_emission_node(node) {
            let lint = self.rustc_converter.to_lint(api_lint);
            let level = self.rustc_cx.lint_level_at_node(lint, id).0;
            self.marker_converter.to_lint_level(level)
        } else {
            Level::Allow
        }
    }

    fn emit_diag(&'ast self, diag: &Diagnostic<'_, 'ast>) {
        let Some(id) = self
            .rustc_converter
            .try_to_hir_id_from_emission_node(diag.node)
        else {
            return;
        };
        let lint = self.rustc_converter.to_lint(diag.lint);
        self.rustc_cx.struct_span_lint_hir(
            lint,
            id,
            self.rustc_converter.to_span(diag.span),
            diag.msg().to_string(),
            |builder| {
                for part in diag.parts.get() {
                    match part {
                        marker_api::diagnostic::DiagnosticPart::Help { msg } => {
                            builder.help(msg.get().to_string());
                        }
                        marker_api::diagnostic::DiagnosticPart::HelpSpan { msg, span } => {
                            builder.span_help(
                                self.rustc_converter.to_span(span),
                                msg.get().to_string(),
                            );
                        }
                        marker_api::diagnostic::DiagnosticPart::Note { msg } => {
                            builder.note(msg.get().to_string());
                        }
                        marker_api::diagnostic::DiagnosticPart::NoteSpan { msg, span } => {
                            builder.span_note(
                                self.rustc_converter.to_span(span),
                                msg.get().to_string(),
                            );
                        }
                        marker_api::diagnostic::DiagnosticPart::Suggestion {
                            msg,
                            span,
                            sugg,
                            app,
                        } => {
                            builder.span_suggestion(
                                self.rustc_converter.to_span(span),
                                msg.get().to_string(),
                                sugg.get().to_string(),
                                self.rustc_converter.to_applicability(*app),
                            );
                        }
                        _ => unreachable!(),
                    }
                }
                builder
            },
        );
    }

    fn item(&'ast self, api_id: ItemId) -> Option<ItemKind<'ast>> {
        let rustc_id = self.rustc_converter.to_item_id(api_id);
        let rust_item = self.rustc_cx.hir().item(rustc_id);
        self.marker_converter.to_item(rust_item)
    }

    fn body(&'ast self, id: BodyId) -> &'ast Body<'ast> {
        let rustc_body = self
            .rustc_cx
            .hir()
            .body(self.rustc_converter.to_body_id(id));
        self.marker_converter.to_body(rustc_body)
    }

    fn resolve_ty_ids(&'ast self, path: &str) -> &'ast [TyDefId] {
        // Caching
        if let Some(ids) = self.resolved_ty_ids.borrow().get(path) {
            return ids;
        }

        // Path splitting and "validation"
        let mut splits = path.split("::");
        let Some(krate_name) = splits.next() else {
            return &[];
        };
        let segs: Vec<_> = splits.collect();
        if segs.is_empty() {
            return &[];
        }
        // This method is only intended to resolve `TyDefId`s, this means we can
        // ignore primitive types and all others which are specificity handled in
        // the `*TyKind` enums. Basically, we only need to find the ids of Enums,
        // Structs, Unions and maybe type aliases.
        //
        // This code is inspired by `clippy_utils::def_path_res` without the special
        // handling for primitive types and other items
        let tcx = self.rustc_cx;
        let krate_name = rustc_span::Symbol::intern(krate_name);
        let additional_krate: &[_] = if krate_name == rustc_span::symbol::kw::Crate {
            &[hir::def_id::LOCAL_CRATE]
        } else {
            &[]
        };
        let krates = tcx
            .crates(())
            .iter()
            .copied()
            .chain(std::iter::once(hir::def_id::LOCAL_CRATE))
            .filter(|id| tcx.crate_name(*id) == krate_name)
            .chain(additional_krate.iter().copied());
        let mut searches: Vec<_> = krates
            .map(rustc_span::def_id::CrateNum::as_def_id)
            .map(|id| hir::def::Res::Def::<hir::def_id::DefId>(tcx.def_kind(id), id))
            .collect();

        let mut rest = &segs[..];
        while let [seg, next_rest @ ..] = rest {
            rest = next_rest;
            let seg = rustc_span::Symbol::intern(seg);
            searches = select_children_with_name(tcx, &searches, seg);
        }

        // Filtering to only take `DefId`s which are also `TyDefId`s
        let ids: Vec<_> = searches
            .into_iter()
            .filter_map(|res| res.opt_def_id())
            .filter(|def_id| {
                matches!(
                    tcx.def_kind(def_id),
                    hir::def::DefKind::Struct
                        | hir::def::DefKind::Union
                        | hir::def::DefKind::Enum
                        | hir::def::DefKind::Trait
                        | hir::def::DefKind::TyAlias
                )
            })
            .map(|def_id| self.marker_converter.to_ty_def_id(def_id))
            .collect();

        // Allocation and caching
        let ids = self.storage.alloc_slice(ids);
        self.resolved_ty_ids
            .borrow_mut()
            .insert(self.storage.alloc_str(path), ids);
        ids
    }

    fn expr_ty(&'ast self, expr: ExprId) -> marker_api::ast::ty::SemTyKind<'ast> {
        let hir_id = self.rustc_converter.to_hir_id(expr);
        self.marker_converter.expr_ty(hir_id)
    }

    fn span(&'ast self, span_id: SpanId) -> &'ast Span<'ast> {
        let rustc_span = self.rustc_converter.to_span_from_id(span_id);
        self.storage
            .alloc(self.marker_converter.to_span(rustc_span))
    }

    fn span_snippet(&self, api_span: &Span<'_>) -> Option<&'ast str> {
        let rust_span = self.rustc_converter.to_span(api_span);
        let snippet = self
            .rustc_cx
            .sess
            .source_map()
            .span_to_snippet(rust_span)
            .ok()?;
        Some(self.storage.alloc_str(&snippet))
    }

    fn span_source(&'ast self, api_span: &Span<'_>) -> marker_api::ast::SpanSource<'ast> {
        let rust_span = self.rustc_converter.to_span(api_span);
        self.marker_converter.to_span_source(rust_span)
    }

    fn span_pos_to_file_loc(
        &'ast self,
        file: &marker_api::ast::FileInfo<'ast>,
        pos: marker_api::ast::SpanPos,
    ) -> Option<marker_api::ast::FilePos<'ast>> {
        self.marker_converter.try_to_span_pos(
            self.rustc_converter.to_syntax_context(file.span_src()),
            self.rustc_converter.to_byte_pos(pos),
        )
    }

    fn span_expn_info(
        &'ast self,
        expn_id: marker_api::ast::ExpnId,
    ) -> Option<&'ast marker_api::ast::ExpnInfo<'ast>> {
        let id = self.rustc_converter.to_expn_id(expn_id);
        self.marker_converter.try_to_expn_info(id)
    }

    fn symbol_str(&'ast self, api_id: SymbolId) -> &'ast str {
        let sym = self.rustc_converter.to_symbol(api_id);
        // The lifetime is fake, as documented in [`rustc_span::Span::as_str()`].
        // It'll definitely live longer than the `'ast` lifetime, it's transmuted to.
        let rustc_str: &str = sym.as_str();
        // # Safety
        // `'ast` is shorter than `'tcx` or any rustc lifetime. This transmute
        // in combination with the comment above is therefore safe.
        let api_str: &'ast str = unsafe { std::mem::transmute(rustc_str) };
        api_str
    }

    fn resolve_method_target(&'ast self, _id: ExprId) -> ItemId {
        todo!()
    }
}

fn select_children_with_name(
    tcx: TyCtxt<'_>,
    search: &[hir::def::Res<hir::def_id::DefId>],
    name: rustc_span::Symbol,
) -> Vec<hir::def::Res<hir::def_id::DefId>> {
    let mut next_search = vec![];

    let mod_def_ids = search.iter().filter_map(rustc_hir::def::Res::mod_def_id);

    for id in mod_def_ids {
        if let Some(local_id) = id.as_local() {
            let hir = tcx.hir();

            let root_mod;
            let item = match hir.find_by_def_id(local_id) {
                Some(hir::Node::Crate(r#mod)) => {
                    root_mod = hir::ItemKind::Mod(r#mod);
                    Some(&root_mod)
                }
                Some(hir::Node::Item(item)) => Some(&item.kind),
                _ => None,
            };

            if let Some(hir::ItemKind::Mod(module)) = item {
                module
                    .item_ids
                    .iter()
                    .filter_map(|&item_id| {
                        if hir.item(item_id).ident.name == name {
                            let def_id = item_id.owner_id.to_def_id();
                            Some(hir::def::Res::Def(tcx.def_kind(def_id), def_id))
                        } else {
                            None
                        }
                    })
                    .collect_into(&mut next_search);
            }
        } else if let hir::def::DefKind::Mod = tcx.def_kind(id) {
            tcx.module_children(id)
                .iter()
                .filter(|item| item.ident.name == name)
                .map(|child| child.res.expect_non_local())
                .collect_into(&mut next_search);
        }
    }

    next_search
}
