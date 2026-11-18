#![cfg_attr(dylint_lib = "general", allow(crate_wide_allow))]
#![cfg_attr(dylint_lib = "supplementary", allow(nonexistent_path_in_comment))]

use assert_cmd::{Command, cargo::cargo_bin_cmd};
use dylint_internal::{env, home::cargo_home, rustup::active_toolchain};
use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

#[ctor::ctor(unsafe)]
fn initialize() {
    unsafe {
        std::env::remove_var(env::CARGO_TERM_COLOR);
    }
}

#[allow(deprecated)]
fn dylint_with_home(home: &Path) -> Command {
    let original_home = std::env::home_dir().unwrap();
    let rustup_home = std::env::var_os(env::RUSTUP_HOME)
        .map_or_else(|| original_home.join(".rustup"), Into::into);
    // dblock: Isolate libraries without rebuilding installed drivers for every test.
    let drivers = installed_driver_directory();
    let mut command = cargo_bin_cmd!("cargo-dylint");
    command
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env(env::CARGO_HOME, cargo_home().unwrap())
        .env(env::RUSTUP_HOME, rustup_home)
        .env_remove(env::DYLINT_CACHE_DIR)
        .env(env::DYLINT_DRIVER_PATH, drivers);
    command
}

fn installed_driver_directory() -> PathBuf {
    static DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| {
        let toolchain =
            active_toolchain(Path::new("../examples/restriction/collapsible_unwrap")).unwrap();
        let driver =
            dylint::driver_builder::get(&dylint::opts::Dylint::default(), &toolchain).unwrap();
        driver
            .parent()
            .and_then(Path::parent)
            .unwrap()
            .to_path_buf()
    });
    DIRECTORY.clone()
}

mod depinfo;
mod dylint_driver_path;
mod fix;
mod library_packages;
mod list;
mod nightly_toolchain;
mod no_deps;
mod package_options;
mod warn;
