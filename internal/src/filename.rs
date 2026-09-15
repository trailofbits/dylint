use std::{env::consts, path::Path};

#[must_use]
pub fn library_plain_filename(name: &str) -> String {
    format!(
        "{}{}{}",
        consts::DLL_PREFIX,
        name.replace('-', "_"),
        consts::DLL_SUFFIX
    )
}

#[must_use]
pub fn parse_plain_path(path: &Path) -> Option<String> {
    let filename = path.file_name()?;
    let s = filename.to_string_lossy();
    let file_stem = s.strip_suffix(consts::DLL_SUFFIX)?;
    let lib_name = file_stem.strip_prefix(consts::DLL_PREFIX)?;
    Some(lib_name.to_owned())
}

/// Returns the filename of a Dylint library.
///
/// # Examples
///
/// ```
/// use dylint_internal::library_filename_with_toolchain;
///
/// #[cfg(target_os = "linux")]
/// assert_eq!(
///     library_filename_with_toolchain("foo", "stable-x86_64-unknown-linux-gnu"),
///     "libfoo@stable-x86_64-unknown-linux-gnu.so"
/// );
///
/// #[cfg(target_os = "macos")]
/// assert_eq!(
///     library_filename_with_toolchain("foo", "stable-x86_64-apple-darwin"),
///     "libfoo@stable-x86_64-apple-darwin.dylib"
/// );
///
/// #[cfg(target_os = "windows")]
/// assert_eq!(
///     library_filename_with_toolchain("foo", "stable-x86_64-pc-windows-msvc"),
///     "foo@stable-x86_64-pc-windows-msvc.dll"
/// );
/// ```
// smoelius: Build a standard rlib, and the filename will use snake case. `library_filename`'s
// behavior is consistent with that.
#[allow(clippy::module_name_repetitions, clippy::uninlined_format_args)]
#[must_use]
pub fn library_filename_with_toolchain(name: &str, toolchain: &str) -> String {
    format!(
        "{}{}@{}{}",
        consts::DLL_PREFIX,
        name.replace('-', "_"),
        toolchain,
        consts::DLL_SUFFIX
    )
}

/// Parses the filename of a Dylint library path into a tuple of (name, toolchain).
///
/// # Examples
///
/// ```
/// use dylint_internal::parse_path_with_toolchain;
/// use std::path::Path;
///
/// #[cfg(target_os = "linux")]
/// assert_eq!(
///     parse_path_with_toolchain(Path::new("libfoo@stable-x86_64-unknown-linux-gnu.so")),
///     Some((
///         String::from("foo"),
///         String::from("stable-x86_64-unknown-linux-gnu")
///     ))
/// );
///
/// #[cfg(target_os = "macos")]
/// assert_eq!(
///     parse_path_with_toolchain(Path::new("libfoo@stable-x86_64-apple-darwin.dylib")),
///     Some((
///         String::from("foo"),
///         String::from("stable-x86_64-apple-darwin")
///     ))
/// );
///
/// #[cfg(target_os = "windows")]
/// assert_eq!(
///     parse_path_with_toolchain(Path::new("foo@stable-x86_64-pc-windows-msvc.dll")),
///     Some((
///         String::from("foo"),
///         String::from("stable-x86_64-pc-windows-msvc")
///     ))
/// );
/// ```
#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn parse_path_with_toolchain(path: &Path) -> Option<(String, String)> {
    let filename = path.file_name()?;
    parse_filename_with_toolchain(&filename.to_string_lossy())
}

#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn parse_filename_with_toolchain(filename: &str) -> Option<(String, String)> {
    let file_stem = filename.strip_suffix(consts::DLL_SUFFIX)?;
    let lib_name_with_toolchain = file_stem.strip_prefix(consts::DLL_PREFIX)?;
    parse_lib_name_with_toolchain(lib_name_with_toolchain)
}

fn parse_lib_name_with_toolchain(lib_name_with_toolchain: &str) -> Option<(String, String)> {
    let (lib_name, toolchain) = lib_name_with_toolchain.split_once('@')?;
    Some((lib_name.to_owned(), toolchain.to_owned()))
}
