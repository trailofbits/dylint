#![feature(iter_collect_into)]
#![feature(let_chains)]
#![feature(lint_reasons)]
#![feature(non_exhaustive_omitted_patterns_lint)]
#![feature(once_cell_try)]
#![feature(rustc_private)]
#![warn(unused_extern_crates)]

use camino::Utf8PathBuf;
use marker_adapter::{Adapter, LintCrateInfo};
use marker_rustc_driver::lint_pass;
use rustc_lint::{LateContext, LateLintPass};
use serde::Deserialize;

dylint_linting::impl_late_lint! {
    /// ### What it does
    /// Runs Marker lints from a Dylint library.
    ///
    /// ### Configuration
    /// - `lint_crates`: A list of [`marker_adapter::LintCrateInfo`]. Each is a struct containing
    ///   two fields, `name` and `path`, which are documented as follows:
    ///   - `name`: The name of the lint crate
    ///   - `path`: The absolute path of the compiled dynamic library, which can be loaded as a lint
    ///     crate
    ///
    /// [`marker_adapter::LintCrateInfo`]: https://docs.rs/marker_adapter/latest/marker_adapter/struct.LintCrateInfo.html
    pub MARKER,
    Warn,
    "Marker lints run from a Dylint library",
    Marker::new()
}

#[derive(Clone, Deserialize)]
struct DeserializableLintCrateInfo {
    pub name: String,
    pub path: Utf8PathBuf,
}

impl From<DeserializableLintCrateInfo> for LintCrateInfo {
    fn from(value: DeserializableLintCrateInfo) -> Self {
        let DeserializableLintCrateInfo { name, path } = value;
        Self { name, path }
    }
}

#[derive(Default, Deserialize)]
struct Config {
    lint_crates: Vec<DeserializableLintCrateInfo>,
}

struct Marker {
    config: Config,
}

impl Marker {
    pub fn new() -> Self {
        Self {
            config: dylint_linting::config_or_default(env!("CARGO_PKG_NAME")),
        }
    }

    fn lint_crates(&self) -> Vec<LintCrateInfo> {
        self.config
            .lint_crates
            .clone()
            .into_iter()
            .map(Into::into)
            .collect()
    }
}

impl<'tcx> LateLintPass<'tcx> for Marker {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        let adapter = Adapter::new(&self.lint_crates()).unwrap();
        lint_pass::process_crate(cx, &adapter);
    }
}
