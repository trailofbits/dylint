#![cfg_attr(dylint_lib = "general", allow(crate_wide_allow))]
#![cfg_attr(dylint_lib = "supplementary", allow(nonexistent_path_in_comment))]

#[ctor::ctor(unsafe)]
fn initialize() {
    unsafe {
        std::env::remove_var(dylint_internal::env::CARGO_TERM_COLOR);
    }
}

mod depinfo;
mod depinfo_dylint_toml;
mod dylint_driver_path;
mod fix;
mod library_packages;
mod list;
mod nightly_toolchain;
mod no_deps;
mod package_options;
mod warn;
