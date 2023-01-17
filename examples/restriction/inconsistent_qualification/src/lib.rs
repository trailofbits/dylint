#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_note;
use if_chain::if_chain;
use rustc_hir::{
    def::{DefKind, Res},
    intravisit::{walk_item, Visitor},
    HirId, Item, ItemKind, Node, Path, UseKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Symbol;

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Checks that a module's items are either imported or qualified with the module's path, but
    /// not both.
    ///
    /// ### Why is this bad?
    /// Mixing the two styles can lead to confusing code.
    ///
    /// ### Known problems
    /// - No exception is made for for qualifications required for disambiguation.
    /// - Re-exports may not be handled correctly.
    ///
    /// ### Example
    /// ```rust
    /// use std::env::var;
    /// fn main() {
    ///     assert_eq!(var("LD_PRELOAD"), Err(std::env::VarError::NotPresent));
    /// }
    /// ```
    /// Instead, either use:
    /// ```rust
    /// use std::env::{var, VarError};
    /// fn main() {
    ///     assert_eq!(var("LD_PRELOAD"), Err(VarError::NotPresent));
    /// }
    /// ```
    /// Or use:
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
        if_chain! {
            // smoelius: On the Dylint source code itself, simply checking
            // `path.span.in_derive_expansion()` isn't sufficient to prevent false positives.
            if !cx
                .tcx
                .hir()
                .parent_iter(hir_id)
                .any(|(hir_id, _)| cx.tcx.hir().span(hir_id).in_derive_expansion());
            let node = cx.tcx.hir().get(hir_id);
            if !matches!(
                node,
                Node::Item(Item {
                    kind: ItemKind::Use(..),
                    ..
                })
            );
            if let Some(def_id) = path.segments.iter().rev().find_map(|segment| {
                if let Res::Def(DefKind::Mod, def_id) = segment.res {
                    Some(def_id)
                } else {
                    None
                }
            });
            then {
                let syms_prefix = cx.get_def_path(def_id);
                // smoelius: Iterate over all enclosing scopes.
                let mut current_hir_id = hir_id;
                loop {
                    let enclosing_scope_hir_id = cx.tcx.hir().get_enclosing_scope(current_hir_id);
                    let mut visitor = UseVisitor {
                        cx,
                        enclosing_scope_hir_id,
                        path,
                        syms_prefix: &syms_prefix,
                    };
                    if let Some(enclosing_scope_hir_id) = enclosing_scope_hir_id {
                        let node = cx.tcx.hir().find(enclosing_scope_hir_id).unwrap();
                        visitor.visit_scope(node);
                        current_hir_id = enclosing_scope_hir_id;
                    } else {
                        let parent_module_local_def_id = cx.tcx.parent_module(hir_id);
                        let parent_module = cx.tcx.hir().get_module(parent_module_local_def_id);
                        visitor.visit_mod(parent_module.0, parent_module.1, parent_module.2);
                        break;
                    }
                }
            }
        }
    }
}

struct UseVisitor<'cx, 'tcx, 'syms> {
    cx: &'cx LateContext<'tcx>,
    enclosing_scope_hir_id: Option<HirId>,
    path: &'cx Path<'tcx>,
    syms_prefix: &'syms [Symbol],
}

// smoelius: `visit_scope` is based on the source of:
// https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/hir/map/struct.Map.html#method.get_enclosing_scope
// Does something similar already exist and I am just not seeing it?
impl<'cx, 'tcx, 'syms> UseVisitor<'cx, 'tcx, 'syms> {
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

impl<'cx, 'tcx, 'syms> Visitor<'tcx> for UseVisitor<'cx, 'tcx, 'syms> {
    type NestedFilter = rustc_middle::hir::nested_filter::All;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.cx.tcx.hir()
    }

    fn visit_item(&mut self, item: &'tcx Item) {
        if_chain! {
            if !item.span.from_expansion();
            if self.cx.tcx.hir().get_enclosing_scope(item.hir_id()) == self.enclosing_scope_hir_id;
            if let ItemKind::Use(path, use_kind) = item.kind;
            // smoelius: An exception is made for trait imports.
            if !path
                .res
                .iter()
                .any(|res| matches!(res, Res::Def(DefKind::Trait, _)));
            let syms = path
                .segments
                .iter()
                .map(|segment| segment.ident.name)
                .collect::<Vec<_>>();
            if match use_kind {
                UseKind::Single => syms[..syms.len() - 1] == *self.syms_prefix,
                UseKind::Glob => syms == self.syms_prefix,
                UseKind::ListStem => false,
            };
            then {
                let prefix = self
                    .syms_prefix
                    .iter()
                    .map(Symbol::as_str)
                    .collect::<Vec<_>>()
                    .join("::");
                span_lint_and_note(
                    self.cx,
                    INCONSISTENT_QUALIFICATION,
                    self.path.span,
                    "inconsistent qualification",
                    Some(item.span),
                    &format!("items from `{prefix}` were imported here"),
                );
            }
        }
        walk_item(self, item);
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(
        env!("CARGO_PKG_NAME"),
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui"),
    );
}
