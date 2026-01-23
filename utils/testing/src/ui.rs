use crate::{example_target, example_targets, initialize, run_example_test, run_tests};
use std::{
    env::current_dir,
    path::{Path, PathBuf},
};

enum Target {
    SrcBase(PathBuf),
    Example(String),
    Examples,
}

#[derive(Clone, Default)]
pub(super) struct Config {
    pub(super) rustc_flags: Vec<String>,
    pub(super) dylint_toml: Option<String>,
}

/// Test builder
pub struct Test {
    name: String,
    target: Target,
    config: Config,
}

impl Test {
    /// Test a library on all source files in a directory (similar to [`ui_test`]).
    ///
    /// [`ui_test`]: crate::ui_test
    #[must_use]
    pub fn src_base(name: &str, src_base: impl AsRef<Path>) -> Self {
        Self::new(name, Target::SrcBase(src_base.as_ref().to_owned()))
    }

    /// Test a library on one example target (similar to [`ui_test_example`]).
    ///
    /// [`ui_test_example`]: crate::ui_test_example
    #[must_use]
    pub fn example(name: &str, example: &str) -> Self {
        Self::new(name, Target::Example(example.to_owned()))
    }

    /// Test a library on all example targets (similar to [`ui_test_examples`]).
    ///
    /// [`ui_test_examples`]: crate::ui_test_examples
    #[must_use]
    pub fn examples(name: &str) -> Self {
        Self::new(name, Target::Examples)
    }

    /// Pass flags to the compiler when running the test.
    pub fn rustc_flags(
        &mut self,
        rustc_flags: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> &mut Self {
        self.config
            .rustc_flags
            .extend(rustc_flags.into_iter().map(|s| s.as_ref().to_owned()));
        self
    }

    /// Set the `dylint.toml` file's contents (for testing configurable libraries).
    pub fn dylint_toml(&mut self, dylint_toml: impl AsRef<str>) -> &mut Self {
        self.config.dylint_toml = Some(dylint_toml.as_ref().to_owned());
        self
    }

    /// Run the test.
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub fn run(&mut self) {
        self.run_immutable();
    }

    fn new(name: &str, target: Target) -> Self {
        Self {
            name: name.to_owned(),
            target,
            config: Config::default(),
        }
    }

    fn run_immutable(&self) {
        let driver = initialize(&self.name).as_ref().unwrap();

        match &self.target {
            Target::SrcBase(src_base) => {
                run_tests(driver, src_base, &self.config);
            }
            Target::Example(example) => {
                let metadata = dylint_internal::cargo::current_metadata().unwrap();
                let current_dir = current_dir().unwrap();
                let package =
                    dylint_internal::cargo::package_with_root(&metadata, &current_dir).unwrap();
                let target = example_target(&package, example).unwrap();

                run_example_test(driver, &metadata, &package, &target, &self.config).unwrap();
            }
            Target::Examples => {
                let metadata = dylint_internal::cargo::current_metadata().unwrap();
                let current_dir = current_dir().unwrap();
                let package =
                    dylint_internal::cargo::package_with_root(&metadata, &current_dir).unwrap();
                let targets = example_targets(&package).unwrap();

                for target in targets {
                    run_example_test(driver, &metadata, &package, &target, &self.config).unwrap();
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // smoelius: Verify that `rustc_flags` compiles when used as intended.
    #[allow(dead_code)]
    fn rustc_flags() {
        let _ = Test::src_base("name", PathBuf::new()).rustc_flags(["--test"]);
    }
}
