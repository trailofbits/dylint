use std::env::consts;

#[must_use]
pub fn library_filename(lib_name: &str, toolchain: &str) -> String {
    format!(
        "{}{}@{}{}",
        consts::DLL_PREFIX,
        lib_name,
        toolchain,
        consts::DLL_SUFFIX
    )
}
