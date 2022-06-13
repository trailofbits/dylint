use std::{env::consts, path::Path};

// smoelius: Build a standard rlib, and the filename will use snake case. `library_filename`'s
// behavior is consistent with that.
#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn library_filename(lib_name: &str, toolchain: &str) -> String {
    format!(
        "{}{}@{}{}",
        consts::DLL_PREFIX,
        lib_name.replace('-', "_"),
        toolchain,
        consts::DLL_SUFFIX
    )
}

#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn parse_path_filename(path: &Path) -> Option<(String, String)> {
    let filename = path.file_name()?;
    parse_filename(&*filename.to_string_lossy())
}

#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn parse_filename(filename: &str) -> Option<(String, String)> {
    let file_stem = filename.strip_suffix(consts::DLL_SUFFIX)?;
    let target_name = file_stem.strip_prefix(consts::DLL_PREFIX)?;
    parse_target_name(target_name)
}

fn parse_target_name(target_name: &str) -> Option<(String, String)> {
    let (lib_name, toolchain) = target_name.split_once('@')?;
    Some((lib_name.to_owned(), toolchain.to_owned()))
}
