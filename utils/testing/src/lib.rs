use anyhow::{anyhow, Result};
use compiletest_rs::{self as compiletest, common::Mode as TestMode};
use dylint_internal::env;
use std::{env::set_var, path::Path};

pub fn ui_test(name: &str, src_base: &Path) {
    let _ = env_logger::builder().try_init();

    dylint_internal::build().success().unwrap();

    let dylint_libs = dylint_libs(name).unwrap();
    let driver = dylint::driver_builder::get(env!("RUSTUP_TOOLCHAIN")).unwrap();

    set_var(env::CLIPPY_DISABLE_DOCS_LINKS, "true");
    set_var(env::DYLINT_LIBS, dylint_libs);
    set_var(
        env::DYLINT_RUSTFLAGS,
        "--emit=metadata -Dwarnings -Zui-testing",
    );

    let config = compiletest::Config {
        mode: TestMode::Ui,
        rustc_path: driver,
        src_base: src_base.to_path_buf(),
        ..compiletest::Config::default()
    };
    compiletest::run_tests(&config);
}

pub fn dylint_libs(name: &str) -> Result<String> {
    let name_toolchain_map = dylint::name_toolchain_map()?;
    let entry = dylint::name_as_lib(&name_toolchain_map, name, false)?;
    let (_, path) = entry.ok_or_else(|| anyhow!("Could not find library"))?;
    let paths = vec![path];
    serde_json::to_string(&paths).map_err(Into::into)
}
