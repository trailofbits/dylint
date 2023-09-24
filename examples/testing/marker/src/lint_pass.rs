use std::cell::OnceCell;

use marker_adapter::{Adapter, AdapterError, LintCrateInfo};
use marker_api::lint::Lint;

use crate::context::{storage::Storage, RustcContext};

thread_local! {
    /// The [`Adapter`] loads the lint crates and is the general interface used by drivers to communicate with lint crates.
    ///
    /// The lint crates have to be loaded before the instantiation of [`RustcLintPass`]
    /// to allow this driver to register the lints before the lint pass starts.
    /// (See [`super::MarkerCallback::config`]). Storing the `Adapter` in a `thread_local`
    /// cell is the easiest solution I could come up with. It should be fine performance wise.
    ///
    /// Storing the [`Adapter`] in a `thread_local` is safe, since rustc is currently only single threaded. This cell will therefore only be constructed once, and this driver will always use the same adapter.
    static ADAPTER: OnceCell<Adapter> = OnceCell::new();
}

pub struct RustcLintPass;

impl RustcLintPass {
    pub fn init_adapter(lint_crates: &[LintCrateInfo]) -> Result<(), AdapterError> {
        ADAPTER.with(move |cell| {
            cell.get_or_try_init(|| Adapter::new(lint_crates))?;
            Ok(())
        })
    }

    pub fn marker_lints() -> Vec<&'static Lint> {
        ADAPTER.with(|adapter| {
            adapter
                .get()
                .unwrap()
                .lint_pass_infos()
                .iter()
                .flat_map(marker_api::LintPassInfo::lints)
                .copied()
                .collect()
        })
    }
}

rustc_lint_defs::impl_lint_pass!(RustcLintPass => []);

impl<'tcx> rustc_lint::LateLintPass<'tcx> for RustcLintPass {
    fn check_crate(&mut self, rustc_cx: &rustc_lint::LateContext<'tcx>) {
        ADAPTER.with(|adapter| {
            process_crate(rustc_cx, adapter.get().unwrap());
        });
    }
}

pub fn process_crate(rustc_cx: &rustc_lint::LateContext<'_>, adapter: &Adapter) {
    let storage = Storage::default();
    process_crate_lifetime(rustc_cx, &storage, adapter);
}

/// This function marks the start of the `'ast` lifetime. The lifetime is defined by the [`Storage`]
/// object.
fn process_crate_lifetime<'ast, 'tcx: 'ast>(
    rustc_cx: &rustc_lint::LateContext<'tcx>,
    storage: &'ast Storage<'ast>,
    adapter: &Adapter,
) {
    let driver_cx = RustcContext::new(rustc_cx.tcx, rustc_cx.lint_store, storage);

    // To support debug printing of AST nodes, as these might sometimes require the
    // context. Note that this only sets the cx for the rustc side. Each lint crate
    // has their own storage for cx.
    marker_api::context::set_ast_cx(driver_cx.ast_cx());

    let krate = driver_cx.marker_converter.to_crate(
        rustc_hir::def_id::LOCAL_CRATE,
        driver_cx.rustc_cx.hir().root_module(),
    );

    adapter.process_krate(driver_cx.ast_cx(), krate);
}
