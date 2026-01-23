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
//! [struct update syntax]: https://doc.rust-lang.org/book/ch05-01-defining-structs.html#creating-instances-from-other-instances-with-struct-update-syntax

#[cfg(feature = "package_options")]
use std::sync::LazyLock;

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default)]
pub struct Dylint {
    pub pipe_stderr: Option<String>,

    pub pipe_stdout: Option<String>,

    pub quiet: bool,

    pub operation: Operation,
}

#[derive(Clone, Debug, Default)]
pub struct LibrarySelection {
    pub all: bool,

    pub branch: Option<String>,

    pub git: Option<String>,

    pub lib_paths: Vec<String>,

    pub libs: Vec<String>,

    pub manifest_path: Option<String>,

    pub no_build: bool,

    pub no_metadata: bool,

    pub paths: Vec<String>,

    pub pattern: Option<String>,

    pub rev: Option<String>,

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

    pub fn library_selection_mut(&mut self) -> &mut LibrarySelection {
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
    fn library_selection_mut(&mut self) -> &mut LibrarySelection {
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
