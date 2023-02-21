use std::{env::consts, path::Path};

/// Returns the filename of a Dylint library.
///
/// # Examples
///
/// ```
/// use dylint_internal::library_filename;
///
/// #[cfg(target_os = "linux")]
/// assert_eq!(
///     library_filename("foo", "stable-x86_64-unknown-linux-gnu"),
///     "libfoo@stable-x86_64-unknown-linux-gnu.so"
/// );
///
/// #[cfg(target_os = "macos")]
/// assert_eq!(
///     library_filename("foo", "stable-x86_64-apple-darwin"),
///     "libfoo@stable-x86_64-apple-darwin.dylib"
/// );
///
/// #[cfg(target_os = "windows")]
/// assert_eq!(
///     library_filename("foo", "stable-x86_64-pc-windows-msvc"),
///     "foo@stable-x86_64-pc-windows-msvc.dll"
/// );
/// ```
// smoelius: Build a standard rlib, and the filename will use snake case. `library_filename`'s
// behavior is consistent with that.
#[allow(clippy::module_name_repetitions, clippy::uninlined_format_args)]
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

/// Parses the filename of a Dylint library path into a tuple of (name, toolchain).
///
/// # Examples
///
/// ```
/// use dylint_internal::parse_path_filename;
/// use std::path::Path;
///
/// #[cfg(target_os = "linux")]
/// assert_eq!(
///     parse_path_filename(Path::new("libfoo@stable-x86_64-unknown-linux-gnu.so")),
///     Some((
///         String::from("foo"),
///         String::from("stable-x86_64-unknown-linux-gnu")
///     ))
/// );
///
/// #[cfg(target_os = "macos")]
/// assert_eq!(
///     parse_path_filename(Path::new("libfoo@stable-x86_64-apple-darwin.dylib")),
///     Some((
///         String::from("foo"),
///         String::from("stable-x86_64-apple-darwin")
///     ))
/// );
///
/// #[cfg(target_os = "windows")]
/// assert_eq!(
///     parse_path_filename(Path::new("foo@stable-x86_64-pc-windows-msvc.dll")),
///     Some((
///         String::from("foo"),
///         String::from("stable-x86_64-pc-windows-msvc")
///     ))
/// );
/// ```
#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn parse_path_filename(path: &Path) -> Option<(String, String)> {
    let filename = path.file_name()?;
    parse_filename(&filename.to_string_lossy())
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
