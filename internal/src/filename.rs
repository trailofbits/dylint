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

pub fn parse_filename(filename: &str) -> Option<(String, String)> {
    let file_stem = filename.strip_suffix(consts::DLL_SUFFIX)?;
    let target_name = file_stem.strip_prefix(consts::DLL_PREFIX)?;
    parse_target_name(target_name)
}

fn parse_target_name(target_name: &str) -> Option<(String, String)> {
    let mut iter = target_name.splitn(2, '@');
    let lib_name = iter.next()?;
    let toolchain = iter.next()?;
    Some((lib_name.to_owned(), toolchain.to_owned()))
}
