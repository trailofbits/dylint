use marker_api::lint::{Lint, MacroReport};

use super::RustcConverter;

impl<'ast, 'tcx> RustcConverter<'ast, 'tcx> {
    #[must_use]
    pub fn to_lint(&self, api_lint: &'static Lint) -> &'static rustc_lint::Lint {
        Self::static_to_lint(api_lint)
    }

    /// This not being a method taking `&self` is a small hack, to allow the creation of `&'static
    /// Lint` instances before the start of the `'ast` lifetime, required by the
    /// [`RustcConverter`].
    ///
    /// When possible, please use [`RustcConverter::to_lint`] instead.
    #[must_use]
    pub fn static_to_lint(api_lint: &'static Lint) -> &'static rustc_lint::Lint {
        super::LINTS_MAP.with(|lints| {
            // This extra value, with the explicit lifetime is needed to make rustc
            // see that it actually has the `'static` lifetime
            let lint: &'static rustc_lint::Lint =
                lints.borrow_mut().entry(api_lint).or_insert_with(move || {
                    // Not extracted to an extra function, as it's very specific
                    let report_in_external_macro = match api_lint.report_in_macro {
                        MacroReport::No => false,
                        MacroReport::All => true,
                        _ => unreachable!(),
                    };

                    Box::leak(Box::new(rustc_lint::Lint {
                        name: api_lint.name,
                        default_level: Self::static_to_lint_level(api_lint.default_level),
                        desc: api_lint.explanation,
                        edition_lint_opts: None,
                        report_in_external_macro,
                        future_incompatible: None,
                        is_plugin: true,
                        feature_gate: None,
                        crate_level_only: false,
                    }))
                });
            lint
        })
    }
}
