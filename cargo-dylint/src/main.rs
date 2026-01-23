use clap::{ArgAction, Parser, crate_version};
use std::{ffi::OsStr, fmt::Debug};

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo", display_name = "cargo")]
struct Opts {
    #[clap(subcommand)]
    subcmd: CargoSubcommand,
}

#[derive(Debug, Parser)]
enum CargoSubcommand {
    Dylint(Dylint),
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Parser)]
#[clap(
    version = crate_version!(),
    args_conflicts_with_subcommands = true,
    after_help = r#"ENVIRONMENT VARIABLES:

DYLINT_DRIVER_PATH (default: $HOME/.dylint_drivers) is the directory where Dylint stores rustc
drivers.

DYLINT_LIBRARY_PATH (default: none) is a colon-separated list of directories where Dylint searches
for libraries.

DYLINT_RUSTFLAGS (default: none) is a space-separated list of flags that Dylint passes to `rustc`
when checking the packages in the workspace.

METADATA EXAMPLE:

    [workspace.metadata.dylint]
    libraries = [
        { git = "https://github.com/trailofbits/dylint", pattern = "examples/*/*" },
        { path = "libs/*" },
    ]
"#,
)]
// smoelius: Please keep the last four fields `args`, `operation`, `lib_sel`, and `output`, in that
// order. Please keep all other fields sorted.
struct Dylint {
    #[clap(long, help = "Automatically apply lint suggestions")]
    fix: bool,

    #[clap(long, help = "Continue if `cargo check` fails")]
    keep_going: bool,

    #[clap(long, help = "Do not check other packages within the workspace")]
    no_deps: bool,

    #[clap(
        action = ArgAction::Append,
        number_of_values = 1,
        short,
        long = "package",
        value_name = "SPEC",
        help = "Package to check"
    )]
    packages: Vec<String>,

    #[clap(long, help = "Check all packages in the workspace")]
    workspace: bool,

    #[clap(last = true, help = "Arguments for `cargo check`")]
    args: Vec<String>,

    #[clap(subcommand)]
    operation: Option<Operation>,

    #[clap(flatten)]
    lib_sel: LibrarySelection,

    #[clap(flatten)]
    output: OutputOptions,
}

#[derive(Debug, Parser)]
enum Operation {
    #[clap(
        about = "List libraries or lints",
        long_about = "If no libraries are named, list the name, toolchain, and location of all \
discovered libraries.

If at least one library is named, list the name, level, and description of all lints in all named \
libraries.

Combine with `--all` to list all lints in all discovered libraries."
    )]
    List {
        #[clap(flatten)]
        lib_sel: LibrarySelection,
    },

    #[clap(
        about = "Create a new library package",
        long_about = "Create a new library package at <PATH>"
    )]
    New {
        #[clap(long, help = "Put the package in its own workspace")]
        isolate: bool,

        #[clap(help = "Path to library package")]
        path: String,
    },

    #[clap(
        about = "Upgrade library package",
        long_about = "Upgrade the library package at <PATH> to the latest version of \
                      `clippy_utils`"
    )]
    Upgrade {
        #[clap(long, hide = true)]
        allow_downgrade: bool,

        #[clap(
            long,
            value_name = "VERSION",
            help = "Upgrade to the version of `clippy_utils` with tag `rust-<VERSION>`"
        )]
        rust_version: Option<String>,

        #[clap(help = "Path to library package")]
        path: Option<String>,

        #[clap(
            help_heading = Some("Experimental"),
            long,
            help = "Try to extract fixes from Clippy repository commits whose date is that of the \
                    un-upgraded toolchain or later",
            default_value = "false",
        )]
        auto_correct: bool,
    },
}

#[derive(Debug, Parser)]
#[cfg_attr(feature = "__clap_headings", clap(next_help_heading = Some("Library Selection")))]
struct LibrarySelection {
    #[clap(long, help = "Load all discovered libraries")]
    all: bool,

    #[clap(
        long,
        requires("git"),
        help = "Branch to use when downloading library packages"
    )]
    branch: Option<String>,

    #[clap(
        long,
        value_name = "URL",
        conflicts_with("paths"),
        help = "Git url containing library packages"
    )]
    git: Option<String>,

    #[clap(
        action = ArgAction::Append,
        number_of_values = 1,
        long = "lib-path",
        value_name = "PATH",
        help = "Library path to load lints from"
    )]
    lib_paths: Vec<String>,

    #[clap(
        action = ArgAction::Append,
        number_of_values = 1,
        long = "lib",
        value_name = "NAME",
        help = "Library name to load lints from. A file with a name of the form \"DLL_PREFIX \
        <NAME> '@' TOOLCHAIN DLL_SUFFIX\" is searched for in the directories listed in \
        DYLINT_LIBRARY_PATH, and in the `target/release` directories produced by building the \
        current workspace's metadata entries (see example below)."
    )]
    libs: Vec<String>,

    #[clap(
        long,
        value_name = "PATH",
        help = "Path to Cargo.toml. Note: if the manifest uses metadata, then `--manifest-path \
                <PATH>` must appear before `--`, not after."
    )]
    manifest_path: Option<String>,

    #[clap(long, help = "Do not build metadata entries")]
    no_build: bool,

    #[clap(long, help = "Ignore metadata entirely")]
    no_metadata: bool,

    #[clap(
        action = ArgAction::Append,
        number_of_values = 1,
        long = "path",
        value_name = "PATH",
        conflicts_with("git"),
        help = "Path containing library packages"
    )]
    paths: Vec<String>,

    #[clap(
        long,
        help = "Subdirectories of the `--git` or `--path` argument containing library packages"
    )]
    pattern: Option<String>,

    #[clap(
        long,
        requires("git"),
        help = "Specific commit to use when downloading library packages"
    )]
    rev: Option<String>,

    #[clap(
        long,
        requires("git"),
        help = "Tag to use when downloading library packages"
    )]
    tag: Option<String>,
}

#[derive(Debug, Parser)]
#[cfg_attr(feature = "__clap_headings", clap(next_help_heading = Some("Output Options")))]
struct OutputOptions {
    #[clap(long, value_name = "PATH", help = "Path to pipe stderr to")]
    pipe_stderr: Option<String>,

    #[clap(long, value_name = "PATH", help = "Path to pipe stdout to")]
    pipe_stdout: Option<String>,

    #[clap(
        global = true,
        short,
        long,
        help = "Do not show warnings or progress running commands besides `cargo check` and \
                `cargo fix`"
    )]
    quiet: bool,
}

impl From<Dylint> for dylint::opts::Dylint {
    fn from(opts: Dylint) -> Self {
        let Dylint {
            fix,
            keep_going,
            no_deps,
            packages,
            workspace,
            args,
            operation,
            mut lib_sel,
            output:
                OutputOptions {
                    pipe_stderr,
                    pipe_stdout,
                    quiet,
                },
        } = opts;
        let operation = match operation {
            None => dylint::opts::Operation::Check({
                dylint::opts::Check {
                    lib_sel: lib_sel.into(),
                    fix,
                    keep_going,
                    no_deps,
                    packages,
                    workspace,
                    args,
                }
            }),
            Some(Operation::List { lib_sel: other }) => {
                lib_sel.absorb(other);
                dylint::opts::Operation::List(dylint::opts::List {
                    lib_sel: lib_sel.into(),
                })
            }
            Some(Operation::New { isolate, path }) => {
                dylint::opts::Operation::New(dylint::opts::New { isolate, path })
            }
            Some(Operation::Upgrade {
                allow_downgrade,
                rust_version,
                path,
                auto_correct,
            }) => dylint::opts::Operation::Upgrade(dylint::opts::Upgrade {
                allow_downgrade,
                rust_version,
                auto_correct,
                path,
            }),
        };
        Self {
            pipe_stderr,
            pipe_stdout,
            quiet,
            operation,
        }
    }
}

macro_rules! option_absorb {
    ($this:expr, $other:expr) => {
        if $other.is_some() {
            assert!(
                $this.is_none(),
                "`--{}` used multiple times",
                stringify!($other).replace("_", "-")
            );
            *$this = $other;
        }
    };
}

impl LibrarySelection {
    pub fn absorb(&mut self, other: Self) {
        let Self {
            all,
            branch,
            git,
            lib_paths,
            libs,
            manifest_path,
            no_build,
            no_metadata,
            paths,
            pattern,
            rev,
            tag,
        } = other;
        self.all |= all;
        option_absorb!(&mut self.branch, branch);
        option_absorb!(&mut self.git, git);
        self.lib_paths.extend(lib_paths);
        self.libs.extend(libs);
        option_absorb!(&mut self.manifest_path, manifest_path);
        self.no_build |= no_build;
        self.no_metadata |= no_metadata;
        self.paths.extend(paths);
        option_absorb!(&mut self.pattern, pattern);
        option_absorb!(&mut self.rev, rev);
        option_absorb!(&mut self.tag, tag);
    }
}

impl From<LibrarySelection> for dylint::opts::LibrarySelection {
    fn from(lib_sel: LibrarySelection) -> Self {
        let LibrarySelection {
            all,
            branch,
            git,
            lib_paths,
            libs,
            manifest_path,
            no_build,
            no_metadata,
            paths,
            pattern,
            rev,
            tag,
        } = lib_sel;
        Self {
            all,
            branch,
            git,
            lib_paths,
            libs,
            manifest_path,
            no_build,
            no_metadata,
            paths,
            pattern,
            rev,
            tag,
        }
    }
}

fn main() -> dylint::ColorizedResult<()> {
    env_logger::try_init().unwrap_or_else(|error| {
        dylint::__warn(
            &dylint::opts::Dylint::default(),
            &format!("`env_logger` already initialized: {error}"),
        );
    });

    let args: Vec<_> = std::env::args_os().collect();

    cargo_dylint(&args)
}

fn cargo_dylint<T: AsRef<OsStr>>(args: &[T]) -> dylint::ColorizedResult<()> {
    match Opts::parse_from(args).subcmd {
        CargoSubcommand::Dylint(opts) => dylint::run(&dylint::opts::Dylint::from(opts)),
    }
    .map_err(dylint::ColorizedError::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Opts::command().debug_assert();
    }
}
