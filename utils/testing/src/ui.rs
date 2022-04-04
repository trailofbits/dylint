use crate::{
    current_metadata, example_target, example_targets, initialize, root_package, run_example_test,
    run_tests,
};
use std::path::{Path, PathBuf};

enum Target {
    SrcBase(PathBuf),
    Example(String),
    Examples,
}

pub struct Test {
    name: String,
    target: Target,
    rustc_flags: Vec<String>,
}

impl Test {
    #[must_use]
    pub fn src_base(name: &str, src_base: &Path) -> Self {
        Self::new(name, Target::SrcBase(src_base.to_owned()))
    }

    #[must_use]
    pub fn example(name: &str, example: &str) -> Self {
        Self::new(name, Target::Example(example.to_owned()))
    }

    #[must_use]
    pub fn examples(name: &str) -> Self {
        Self::new(name, Target::Examples)
    }

    pub fn rustc_flags(
        &mut self,
        rustc_flags: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> &mut Self {
        self.rustc_flags
            .extend(rustc_flags.into_iter().map(|s| s.as_ref().to_owned()));
        self
    }

    pub fn run(&mut self) {
        self.run_immutable();
    }

    fn new(name: &str, target: Target) -> Self {
        Self {
            name: name.to_owned(),
            target,
            rustc_flags: Vec::new(),
        }
    }

    fn run_immutable(&self) {
        let driver = initialize(&self.name).unwrap();

        match &self.target {
            Target::SrcBase(src_base) => {
                run_tests(driver, src_base, self.rustc_flags.iter());
            }
            Target::Example(example) => {
                let metadata = current_metadata().unwrap();
                let package = root_package(&metadata).unwrap();
                let target = example_target(&package, example).unwrap();

                run_example_test(
                    driver,
                    &metadata,
                    &package,
                    &target,
                    self.rustc_flags.iter(),
                )
                .unwrap();
            }
            Target::Examples => {
                let metadata = current_metadata().unwrap();
                let package = root_package(&metadata).unwrap();
                let targets = example_targets(&package).unwrap();

                for target in targets {
                    run_example_test(
                        driver,
                        &metadata,
                        &package,
                        &target,
                        self.rustc_flags.iter(),
                    )
                    .unwrap();
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
        let _ = Test::src_base("name", &PathBuf::new()).rustc_flags(["--test"]);
    }
}
