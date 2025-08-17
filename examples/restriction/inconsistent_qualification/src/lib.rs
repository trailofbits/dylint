#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_note;
use rustc_hir::{
    HirId, Item, ItemKind, Node, OwnerNode, Path, PathSegment, UseKind, UsePath,
    def::{DefKind, Res},
    def_id::{CRATE_DEF_ID, DefId},
    intravisit::{Visitor, walk_item},
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks that a module's items are either imported or qualified with the module's path, but
    /// not both.
    ///
    /// ### Why is this bad?
    ///
    /// Mixing the two styles can lead to confusing code.
    ///
    /// ### Known problems
    ///
    /// - No exception is made for for qualifications required for disambiguation.
    /// - Re-exports may not be handled correctly.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use std::env::var;
    /// fn main() {
    ///     assert_eq!(var("LD_PRELOAD"), Err(std::env::VarError::NotPresent));
    /// }
    /// ```
    ///
    /// Instead, either use:
    ///
    /// ```rust
    /// use std::env::{var, VarError};
    /// fn main() {
    ///     assert_eq!(var("LD_PRELOAD"), Err(VarError::NotPresent));
    /// }
    /// ```
    ///
    /// Or use:
    ///
    /// ```rust
    /// fn main() {
    ///     assert_eq!(
    ///         std::env::var("LD_PRELOAD"),
    ///         Err(std::env::VarError::NotPresent)
    ///     );
    /// }
    /// ```
    pub INCONSISTENT_QUALIFICATION,
    Warn,
    "inconsistent qualification of module items"
}

impl<'tcx> LateLintPass<'tcx> for InconsistentQualification {
    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &Path<'tcx>, hir_id: HirId) {
        // smoelius: On the Dylint source code itself, simply checking
        // `path.span.in_derive_expansion()` isn't sufficient to prevent false positives.
        if !path.span.from_expansion()
            && !cx
                .tcx
                .hir_parent_iter(hir_id)
                .any(|(hir_id, _)| cx.tcx.hir_span(hir_id).in_derive_expansion())
            && let node = cx.tcx.hir_node(hir_id)
            && !matches!(
                node,
                Node::Item(Item {
                    kind: ItemKind::Use(..),
                    ..
                })
            )
            && let Some(mod_def_id) = path
                .segments
                .iter()
                .rev()
                .find_map(|segment| segment.res.mod_def_id())
        {
            let syms_mod = cx.get_def_path(mod_def_id);
            // smoelius: Iterate over all enclosing scopes.
            let mut current_hir_id = hir_id;
            loop {
                let enclosing_scope_hir_id = cx.tcx.hir_get_enclosing_scope(current_hir_id);
                let mut visitor = UseVisitor {
                    cx,
                    enclosing_scope_hir_id,
                    path,
                    syms_mod: &syms_mod,
                    diagnostic_emitted: false,
                };
                if let Some(enclosing_scope_hir_id) = enclosing_scope_hir_id {
                    let node = cx.tcx.hir_node(enclosing_scope_hir_id);
                    visitor.visit_scope(node);
                    current_hir_id = enclosing_scope_hir_id;
                    if visitor.diagnostic_emitted {
                        break;
                    }
                } else {
                    let parent_module_local_def_id = cx.tcx.parent_module(hir_id);
                    let parent_module = cx.tcx.hir_get_module(parent_module_local_def_id);
                    visitor.visit_mod(parent_module.0, parent_module.1, parent_module.2);
                    break;
                }
            }
        }
    }
}

struct UseVisitor<'cx, 'tcx, 'syms> {
    cx: &'cx LateContext<'tcx>,
    enclosing_scope_hir_id: Option<HirId>,
    path: &'cx Path<'tcx>,
    syms_mod: &'syms [Symbol],
    diagnostic_emitted: bool,
}

// smoelius: `visit_scope` is based on the source of:
// https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/hir/map/struct.Map.html#method.get_enclosing_scope
// Does something similar already exist and I am just not seeing it?
impl<'tcx> UseVisitor<'_, 'tcx, '_> {
    fn visit_scope(&mut self, node: Node<'tcx>) {
        match node {
            Node::Item(item) => self.visit_item(item),
            Node::ForeignItem(foreign_item) => self.visit_foreign_item(foreign_item),
            Node::TraitItem(trait_item) => self.visit_trait_item(trait_item),
            Node::ImplItem(impl_item) => self.visit_impl_item(impl_item),
            Node::Block(block) => self.visit_block(block),
            _ => {
                panic!("Unexpected node: {node:?}")
            }
        }
    }
}

enum PathMatch<'hir> {
    Prefix(&'hir [PathSegment<'hir>]),
    Mod,
}

impl<'tcx> Visitor<'tcx> for UseVisitor<'_, 'tcx, '_> {
    type NestedFilter = rustc_middle::hir::nested_filter::All;

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }

    fn visit_item(&mut self, item: &'tcx Item) {
        if !item.span.from_expansion()
            // smoelius: Ignore underscore imports.
            && item.kind.ident().is_none_or(|name| name.as_str() != "_")
            && self.cx.tcx.hir_get_enclosing_scope(item.hir_id()) == self.enclosing_scope_hir_id
            && let ItemKind::Use(use_path, use_kind) = item.kind
            && let local_owner_path = if use_path.res.iter().flatten().copied().any(is_local) {
                let local_def_id = self
                    .enclosing_scope_hir_id
                    .and_then(|hir_id| get_owner(self.cx.tcx, hir_id))
                    .map_or(CRATE_DEF_ID, |owner| owner.def_id().def_id);
                self.cx.get_def_path(local_def_id.into())
            } else {
                Vec::new()
            }
            && let syms = local_owner_path
                .into_iter()
                .chain(use_path.segments.iter().map(|segment| segment.ident.name))
                .collect::<Vec<_>>()
            && let Some(path_match) = {
                match use_kind {
                    UseKind::Single(_)=> {
                        if let Some(matched_prefix) = match_path_prefix(use_path, self.path) {
                            Some(PathMatch::Prefix(matched_prefix))
                        } else if syms[..syms.len() - 1] == *self.syms_mod {
                            Some(PathMatch::Mod)
                        } else {
                            None
                        }
                    }
                    UseKind::Glob => {
                        if syms == self.syms_mod {
                            Some(PathMatch::Mod)
                        } else {
                            None
                        }
                    }
                    UseKind::ListStem => None,
                }
            }
            && let use_path_is_trait = use_path
                .res
                .iter()
                .flatten()
                .any(|res| matches!(res, Res::Def(DefKind::Trait, _)))
            // smoelius: If `use_path` corresponds to a trait, then it must match some prefix of
            // `self.path` exactly for a warning to be emitted.
            && (!use_path_is_trait || matches!(path_match, PathMatch::Prefix(_)))
        {
            let (span, msg) = match path_match {
                PathMatch::Prefix(matched_prefix) => {
                    let span = matched_prefix
                        .first()
                        .unwrap()
                        .ident
                        .span
                        .with_hi(matched_prefix.last().unwrap().ident.span.hi());
                    let path =
                        path_to_string(matched_prefix.iter().map(|segment| &segment.ident.name));
                    (span, format!("`{path}` was imported here"))
                }
                PathMatch::Mod => {
                    let path = path_to_string(self.syms_mod);
                    (
                        self.path.span,
                        format!("items from `{path}` were imported here"),
                    )
                }
            };
            span_lint_and_note(
                self.cx,
                INCONSISTENT_QUALIFICATION,
                span,
                "inconsistent qualification",
                Some(item.span),
                msg,
            );
            self.diagnostic_emitted = true;
        }
        walk_item(self, item);
    }
}

fn is_local(res: Res) -> bool {
    res.opt_def_id().is_some_and(DefId::is_local)
}

fn get_owner(tcx: TyCtxt<'_>, hir_id: HirId) -> Option<OwnerNode<'_>> {
    std::iter::once(tcx.hir_node(hir_id))
        .chain(tcx.hir_parent_iter(hir_id).map(|(_, node)| node))
        .find_map(Node::as_owner)
}

fn match_path_prefix<'hir>(
    use_path: &'hir UsePath<'_>,
    path: &'hir Path<'hir>,
) -> Option<&'hir [PathSegment<'hir>]> {
    // smoelius: `skip(1)` to prevent matching `path`'s first segment.
    for (i, segment) in path.segments.iter().enumerate().skip(1).rev() {
        if use_path
            .res
            .iter()
            .flatten()
            .any(|res| res.opt_def_id().is_some() && res.opt_def_id() == segment.res.opt_def_id())
        {
            return Some(&path.segments[..=i]);
        }
    }
    None
}

fn path_to_string<'hir>(path: impl IntoIterator<Item = &'hir Symbol>) -> String {
    path.into_iter()
        .map(Symbol::as_str)
        .collect::<Vec<_>>()
        .join("::")
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
