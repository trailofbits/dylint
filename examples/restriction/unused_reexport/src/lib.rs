#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::Applicability;
use rustc_hir::{
    HirId, Item, ItemKind, Path, UseKind,
    def::{DefKind, Res},
    def_id::{CrateNum, DefId, LocalModId},
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{TyCtxt, Visibility};
use rustc_span::Span;
use std::collections::{HashMap, HashSet, VecDeque};

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Finds paths to dependency items that another direct dependency publicly reexports.
    /// Imports suppress repeated diagnostics for the same definition in their enclosing module.
    ///
    /// ### Why is this bad?
    ///
    /// Using reexports can avoid coupling code to implementation dependencies. Whether this is
    /// desirable depends on the public API contract of the reexporting crate.
    ///
    /// ### Known problems
    ///
    /// Only dependencies loaded by rustc and supplied through the extern prelude are searched.
    /// Macro-generated paths, glob imports, and associated items are not checked.
    /// Suggestions require review: reexports can change between dependency versions.
    ///
    /// ### Example
    ///
    /// If `facade` publicly reexports `origin::Thing`, use `facade::Thing` instead of
    /// `origin::Thing`.
    pub UNUSED_REEXPORT,
    Warn,
    "a direct dependency provides an unused public reexport",
    UnusedReexport::default()
}

#[derive(Default)]
struct UnusedReexport {
    roots: HashMap<String, CrateNum>,
    exports: HashMap<DefId, Vec<(CrateNum, String)>>,
    findings: Vec<Finding>,
}

struct Finding {
    hir_id: HirId,
    module: LocalModId,
    def_id: DefId,
    span: Span,
    replacement: String,
    import: bool,
    editable: bool,
}

impl<'tcx> LateLintPass<'tcx> for UnusedReexport {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        // Cargo's extern name can differ from the crate's declared name. Match artifacts,
        // not crate names, to keep renamed dependencies and multiple versions distinct.
        for (name, entry) in cx.tcx.sess.opts.externs.iter() {
            if !entry.add_prelude {
                continue;
            }
            for &cnum in cx.tcx.crates(()) {
                let source = cx.tcx.used_crate_source(cnum);
                let matches = entry.files().map_or_else(
                    || cx.tcx.crate_name(cnum).as_str() == name,
                    |mut files| {
                        files.any(|file| {
                            source.paths().any(|path| {
                                path == file.original()
                                    || std::fs::canonicalize(path)
                                        .is_ok_and(|path| &path == file.canonicalized())
                            })
                        })
                    },
                );
                if matches {
                    self.roots.insert(name.clone(), cnum);
                    collect_exports(
                        cx.tcx,
                        cnum.as_def_id(),
                        cnum,
                        &format!("::{}", identifier(name)),
                        &mut HashSet::new(),
                        &mut self.exports,
                    );
                }
            }
        }
        for alternatives in self.exports.values_mut() {
            alternatives.sort_by(|a, b| {
                (a.1.matches("::").count(), &a.1).cmp(&(b.1.matches("::").count(), &b.1))
            });
            alternatives.dedup();
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::Use(path, UseKind::Single(binding)) = &item.kind {
            for res in path.res.iter().flatten() {
                self.consider(
                    cx,
                    path.segments,
                    *res,
                    path.span,
                    item.hir_id(),
                    Some(*binding),
                );
            }
        }
    }

    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &Path<'tcx>, hir_id: HirId) {
        self.consider(cx, path.segments, path.res, path.span, hir_id, None);
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Defer diagnostics so an import also suppresses paths appearing before it.
        let imports: HashSet<_> = self
            .findings
            .iter()
            .filter(|f| {
                f.import
                    && cx
                        .tcx
                        .lint_level_spec_at_node(UNUSED_REEXPORT, f.hir_id)
                        .level()
                        != rustc_session::lint::Level::Allow
            })
            .map(|f| (f.module, f.def_id))
            .collect();
        let mut emitted = HashSet::new();
        for f in &self.findings {
            if (!f.import && imports.contains(&(f.module, f.def_id)))
                || !emitted.insert((f.span, f.replacement.clone()))
            {
                continue;
            }
            clippy_utils::diagnostics::span_lint_hir_and_then(
                cx,
                UNUSED_REEXPORT,
                f.hir_id,
                f.span,
                "this path can be replaced with a dependency's public reexport",
                |diag| {
                    if f.editable {
                        diag.span_suggestion(
                            f.span,
                            "use the reexport",
                            &f.replacement,
                            Applicability::MaybeIncorrect,
                        );
                    } else {
                        diag.help(format!(
                            "import `{}` separately from this use tree",
                            f.replacement
                        ));
                    }
                },
            );
        }
    }
}

impl UnusedReexport {
    fn consider(
        &mut self,
        cx: &LateContext<'_>,
        segments: &[rustc_hir::PathSegment<'_>],
        res: Res,
        span: Span,
        hir_id: HirId,
        import: Option<rustc_span::Ident>,
    ) {
        if span.from_expansion()
            || cx
                .tcx
                .hir_parent_iter(hir_id)
                .any(|(parent, _)| cx.tcx.hir_span(parent).in_derive_expansion())
            || segments.len() < 2
        {
            return;
        }
        let Some(first) = segments
            .iter()
            .find(|s| s.ident.name.as_str() != "{{root}}")
        else {
            return;
        };
        let Some(&origin) = self.roots.get(first.ident.name.as_str()) else {
            return;
        };
        if first.res != Res::Def(DefKind::Mod, origin.as_def_id()) {
            return;
        }
        let Res::Def(_, def_id) = res else { return };
        if def_id.is_local() {
            return;
        }
        let Some(alternatives) = self.exports.get(&def_id) else {
            return;
        };
        let Some((_, replacement)) = alternatives.iter().find(|(root, _)| *root != origin) else {
            return;
        };
        let Some(last) = segments.last() else { return };
        let mut replacement = replacement.clone();
        // Lowered use trees include inherited segments whose spans are outside the leaf.
        // They need restructuring, rather than a replacement of the leaf's text.
        let editable = span.contains(first.ident.span);
        if let Some(binding) = import
            && (!editable || binding.span == last.ident.span)
            && replacement.rsplit("::").next() != Some(identifier(binding.name.as_str()).as_str())
        {
            replacement.push_str(" as ");
            replacement.push_str(&identifier(binding.name.as_str()));
        }
        self.findings.push(Finding {
            hir_id,
            module: cx.tcx.parent_module(hir_id),
            def_id,
            span: span.with_hi(last.ident.span.hi()),
            replacement,
            import: import.is_some(),
            editable,
        });
    }
}

fn collect_exports(
    tcx: TyCtxt<'_>,
    module: DefId,
    root: CrateNum,
    prefix: &str,
    visited: &mut HashSet<DefId>,
    exports: &mut HashMap<DefId, Vec<(CrateNum, String)>>,
) {
    // Breadth-first traversal chooses the shortest path to each module and prevents
    // reexport cycles (or many aliases of one module) from multiplying the work.
    let mut queue = VecDeque::from([(module, prefix.to_owned())]);
    while let Some((module, prefix)) = queue.pop_front() {
        if !visited.insert(module) {
            continue;
        }
        let mut children: Vec<_> = tcx.module_children(module).iter().collect();
        children.sort_by_key(|child| identifier(child.ident.name.as_str()));
        for child in children {
            if child.vis != Visibility::Public {
                continue;
            }
            if let Res::Def(kind, def_id) = child.res {
                let path = format!("{prefix}::{}", identifier(child.ident.name.as_str()));
                if def_id.krate != root {
                    exports
                        .entry(def_id)
                        .or_default()
                        .push((root, path.clone()));
                }
                if kind == DefKind::Mod {
                    queue.push_back((def_id, path));
                }
            }
        }
    }
}

fn identifier(name: &str) -> String {
    let symbol = rustc_span::Symbol::intern(name);
    if symbol.is_reserved(|| rustc_span::edition::Edition::Edition2024) {
        format!("r#{name}")
    } else {
        name.to_owned()
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
