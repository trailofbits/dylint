//! Warning: Addition of public fields to the structs in this module is considered a non-breaking
//! change.
//!
//! For this reason, we recommend using [struct update syntax] when initializing instances of the
//! structs in this module.
//!
//! Example:
//!
//! ```
//! # use crate::dylint::opts::Dylint;
//! let opts = Dylint {
//!     quiet: true,
//!     ..Default::default()
//! };
//! ```
//!
//! [struct update syntax]:
//!   https://doc.rust-lang.org/book/ch05-01-defining-structs.html#creating-instances-from-other-instances-with-struct-update-syntax

#[cfg(feature = "package_options")]
use std::sync::LazyLock;

/// Options mirroring the fields of the same names in `cargo-dylint`'s `Dylint` struct.
///
/// Library selection and operation-specific options are grouped under [`Operation`].
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default)]
pub struct Dylint {
    /// Refuse to build a missing, outdated, or unverifiable compiler driver.
    ///
    /// Defaults to `false`. Existing drivers use the normal version policy. To prevent building
    /// lint libraries, use [`LibrarySelection::no_build`].
    pub no_driver_build: bool,

    pub pipe_stderr: Option<String>,

    pub pipe_stdout: Option<String>,

    pub quiet: bool,

    pub operation: Operation,
}

/// Options mirroring the fields of the same names in `cargo-dylint`'s `LibrarySelection` struct.
///
/// The deprecated [`pattern`](Self::pattern) field is retained for compatibility.
// smoelius: Please try to keep these comments in sync with the corresponding option descriptions.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default)]
pub struct LibrarySelection {
    /// Load all discovered libraries.
    pub all: bool,

    /// Branch to use when downloading library packages.
    pub branch: Option<String>,

    /// Exit with an error if no libraries are found; requires a library selection (`--all`,
    /// `--git`, `--lib`, `--lib-path`, or `--path`).
    pub fail_on_no_libraries: bool,

    /// Git url containing library packages.
    pub git: Option<String>,

    /// Library path to load lints from.
    pub lib_paths: Vec<String>,

    /// Library name to load lints from. A file with a name of the form
    /// `"DLL_PREFIX <NAME> '@' TOOLCHAIN DLL_SUFFIX"` is searched for in the directories listed in
    /// `DYLINT_LIBRARY_PATH`, and in the `target/release` directories produced by building the
    /// current workspace's metadata entries.
    pub libs: Vec<String>,

    /// Path to Cargo.toml. Note: if the manifest uses metadata, then `--manifest-path <PATH>` must
    /// appear before `--`, not after.
    pub manifest_path: Option<String>,

    /// Do not build metadata entries.
    pub no_build: bool,

    /// Ignore metadata entirely.
    pub no_metadata: bool,

    /// Path containing library packages.
    pub paths: Vec<String>,

    /// A subdirectory pattern to select library packages, combined with
    /// [`patterns`](Self::patterns).
    #[deprecated(note = "use `patterns`")]
    pub pattern: Option<String>,

    /// Subdirectories of the `--git` or `--path` argument containing library packages.
    pub patterns: Vec<String>,

    /// Specific commit to use when downloading library packages.
    pub rev: Option<String>,

    /// Tag to use when downloading library packages.
    pub tag: Option<String>,
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Operation {
    Check(Check),
    List(List),
    #[cfg(feature = "package_options")]
    New(New),
    #[cfg(feature = "package_options")]
    Upgrade(Upgrade),
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default)]
pub struct Check {
    pub lib_sel: LibrarySelection,

    pub fix: bool,

    pub keep_going: bool,

    pub no_deps: bool,

    pub packages: Vec<String>,

    pub workspace: bool,

    pub args: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct List {
    pub lib_sel: LibrarySelection,
}

#[cfg(feature = "package_options")]
#[derive(Clone, Debug, Default)]
pub struct New {
    pub isolate: bool,

    pub path: String,
}

#[cfg(feature = "package_options")]
#[derive(Clone, Debug, Default)]
pub struct Upgrade {
    pub allow_downgrade: bool,

    pub rust_version: Option<String>,

    pub auto_correct: bool,

    pub path: Option<String>,
}

impl Dylint {
    #[must_use]
    pub const fn has_library_selection(&self) -> bool {
        self.operation.has_library_selection()
    }

    #[must_use]
    pub fn library_selection(&self) -> &LibrarySelection {
        self.operation.library_selection()
    }

    pub const fn library_selection_mut(&mut self) -> &mut LibrarySelection {
        self.operation.library_selection_mut()
    }

    pub(crate) fn git_or_path(&self) -> bool {
        self.library_selection().git_or_path()
    }
}

impl LibrarySelection {
    pub(crate) const fn git_or_path(&self) -> bool {
        self.git.is_some() || !self.paths.is_empty()
    }

    /// Returns `true` if `all`, `libs`, or `lib_paths` indicates a library selection.
    ///
    /// `git` and `paths` are not considered. For the `Check` and `List` operations, `run` folds
    /// them into `all` before any caller reaches this method.
    pub(crate) const fn selects_libraries(&self) -> bool {
        self.all || !self.libs.is_empty() || !self.lib_paths.is_empty()
    }
}

#[cfg(feature = "package_options")]
static LIBRARY_SELECTION: LazyLock<LibrarySelection> = LazyLock::new(LibrarySelection::default);

impl Operation {
    const fn has_library_selection(&self) -> bool {
        match self {
            Self::Check(_) | Self::List(_) => true,
            #[cfg(feature = "package_options")]
            Self::New(_) | Self::Upgrade(_) => false,
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    fn library_selection(&self) -> &LibrarySelection {
        match self {
            Self::Check(check) => &check.lib_sel,
            Self::List(list) => &list.lib_sel,
            #[cfg(feature = "package_options")]
            Self::New(_) | Self::Upgrade(_) => {
                if cfg!(debug_assertions) {
                    eprintln!(
                        "[{}:{}] {}",
                        file!(),
                        line!(),
                        "`library_selection` called on an `Operation` with no `LibrarySelection` \
                         field"
                    );
                    eprintln!("{}", std::backtrace::Backtrace::force_capture());
                }
                &LIBRARY_SELECTION
            }
        }
    }

    #[allow(clippy::panic)]
    const fn library_selection_mut(&mut self) -> &mut LibrarySelection {
        match self {
            Self::Check(check) => &mut check.lib_sel,
            Self::List(list) => &mut list.lib_sel,
            #[cfg(feature = "package_options")]
            Self::New(_) | Self::Upgrade(_) => {
                panic!(
                    "`library_selection_mut` called on an `Operation` with no `LibrarySelection` \
                     field"
                )
            }
        }
    }
}

impl Default for Operation {
    fn default() -> Self {
        Self::Check(Check::default())
    }
}
