use clap::{crate_version, Parser};
use dylint_internal::env::{self, enabled};
use std::{
    ffi::{OsStr, OsString},
    fmt::Debug,
};

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Debug, Parser)]
enum SubCommand {
    Dylint(Dylint),
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Parser)]
#[clap(
    version = crate_version!(),
    after_help = r#"ENVIRONMENT VARIABLES:

DYLINT_DRIVER_PATH (default: $HOME/.dylint_drivers) is the directory where Dylint stores rustc
drivers.

DYLINT_LIBRARY_PATH (default: none) is a colon-separated list of directories where Dylint searches
for libraries.

DYLINT_RUSTFLAGS (default: none) is a space-separated list of flags that Dylint passes to `rustc`.

METADATA EXAMPLE:

    [workspace.metadata.dylint]
    libraries = [
        { git = "https://github.com/trailofbits/dylint", pattern = "examples/*" },
        { path = "libs/*" },
    ]
"#,
)]
pub struct Dylint {
    #[clap(long, help = "Load all discovered libraries")]
    pub all: bool,

    #[clap(long, help = "Automatically apply lint suggestions")]
    pub fix: bool,

    #[clap(long, hide = true)]
    pub isolate: bool,

    #[clap(long, help = "Continue if `cargo check` fails")]
    pub keep_going: bool,

    #[clap(
        multiple_occurrences = true,
        number_of_values = 1,
        long = "lib",
        value_name = "name",
        help = "Library name to load lints from. A file with a name of the form \"DLL_PREFIX \
        <name> '@' TOOLCHAIN DLL_SUFFIX\" is searched for in the directories listed in \
        DYLINT_LIBRARY_PATH, and in the `target/release` directories produced by building the \
        current workspace's metadata entries (see example below)."
    )]
    pub libs: Vec<String>,

    #[clap(
        long,
        help = "If no libraries are named, list the name, toolchain, and location of all \
        discovered libraries. If at least one library is named, list the name, level, and \
        description of all lints in all named libraries. Combine with `--all` to list all \
        lints in all discovered libraries."
    )]
    pub list: bool,

    #[clap(
        long,
        value_name = "path",
        help = "Path to Cargo.toml. Note: if the manifest uses metadata, then \
        `--manifest-path <path>` must appear before `--`, not after."
    )]
    pub manifest_path: Option<String>,

    #[clap(
        long = "new",
        value_name = "path",
        help = "Create a new library package at <path>. Add `--isolate` to put the package in its \
        own workspace."
    )]
    pub new_path: Option<String>,

    #[clap(long, help = "Do not build metadata entries")]
    pub no_build: bool,

    #[clap(long, help = "Ignore metadata entirely")]
    pub no_metadata: bool,

    #[clap(
        multiple_occurrences = true,
        number_of_values = 1,
        short,
        long = "package",
        value_name = "spec",
        help = "Package to check"
    )]
    pub packages: Vec<String>,

    #[clap(
        multiple_occurrences = true,
        number_of_values = 1,
        long = "path",
        value_name = "path",
        help = "Library path to load lints from"
    )]
    pub paths: Vec<String>,

    #[clap(
        short,
        long,
        help = "Do not show warnings or progress running commands besides `cargo check`"
    )]
    pub quiet: bool,

    #[clap(long, hide = true)]
    pub rust_version: Option<String>,

    #[clap(
        long = "upgrade",
        value_name = "path",
        help = "Upgrade the library package at <path> to the latest version of `clippy_utils`. \
        Add `--rust-version <version>` to upgrade to the version with tag `rust-<version>`."
    )]
    pub upgrade_path: Option<String>,

    #[clap(long, help = "Check all packages in the workspace")]
    pub workspace: bool,

    #[clap(
        help = "Libraries to load lints from. Each <name> is searched for as described under \
        `--lib`. If no library is found, <name> is treated as path. To avoid ambiguity, use \
        `--lib` or `--path`."
    )]
    pub names: Vec<String>,

    #[clap(last = true, help = "Arguments for `cargo check`")]
    pub args: Vec<String>,
}

impl From<Dylint> for dylint::Dylint {
    fn from(opts: Dylint) -> Self {
        let Dylint {
            all,
            fix,
            isolate,
            keep_going,
            libs,
            list,
            manifest_path,
            new_path,
            no_build,
            no_metadata,
            packages,
            paths,
            quiet,
            rust_version,
            upgrade_path,
            workspace,
            names,
            args,
        } = opts;
        Self {
            all,
            fix,
            isolate,
            keep_going,
            libs,
            list,
            manifest_path,
            new_path,
            no_build,
            no_metadata,
            packages,
            paths,
            quiet,
            rust_version,
            upgrade_path,
            workspace,
            names,
            args,
        }
    }
}

pub fn main() -> dylint::ColorizedResult<()> {
    env_logger::init();

    let args: Vec<_> = std::env::args().map(OsString::from).collect();

    let result = cargo_dylint(&args);

    if result.is_err() && enabled(env::RUST_BACKTRACE) {
        eprintln!(
            "If you don't see a backtrace below, it could be because `cargo-dylint` wasn't built \
            with a nightly compiler."
        );
    }

    result
}

fn cargo_dylint<T: AsRef<OsStr>>(args: &[T]) -> dylint::ColorizedResult<()> {
    match Opts::parse_from(args).subcmd {
        SubCommand::Dylint(opts) => dylint::run(&dylint::Dylint::from(opts)),
    }
    .map_err(dylint::ColorizedError::new)
}
