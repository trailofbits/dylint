use clap::{crate_version, Clap};
use dylint_internal::env::{self, enabled};
use std::{
    ffi::{OsStr, OsString},
    fmt::Debug,
};

#[derive(Clap, Debug)]
#[clap(bin_name = "cargo")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap, Debug)]
enum SubCommand {
    Dylint(Dylint),
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clap, Debug)]
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
    #[clap(long, about = "Load all discovered libraries")]
    pub all: bool,

    #[clap(
        multiple = true,
        number_of_values = 1,
        long = "lib",
        value_name = "name",
        about = "Library name to load lints from. A file with a name of the form \"DLL_PREFIX \
        <name> '@' TOOLCHAIN DLL_SUFFIX\" is searched for in the directories listed in \
        DYLINT_LIBRARY_PATH, and in the `target/release` directories produced by building the \
        current workspace's metadata entries (see example below)."
    )]
    pub libs: Vec<String>,

    #[clap(
        long,
        about = "If no libraries are named, list the name, toolchain, and location of all \
        discovered libraries. If at least one library is named, list the name, level, and \
        description of all lints in all named libraries. Combine with `--all` to list all \
        lints in all discovered libraries."
    )]
    pub list: bool,

    #[clap(
        long,
        value_name = "path",
        about = "Path to Cargo.toml. Note: if the manifest uses metadata, then \
        `--manifest-path <path>` must appear before `--`, not after."
    )]
    pub manifest_path: Option<String>,

    #[clap(long, about = "Do not build metadata entries")]
    pub no_build: bool,

    #[clap(long, about = "Ignore metadata entirely")]
    pub no_metadata: bool,

    #[clap(
        multiple = true,
        number_of_values = 1,
        short,
        long = "package",
        value_name = "spec",
        about = "Package to check"
    )]
    pub packages: Vec<String>,

    #[clap(
        multiple = true,
        number_of_values = 1,
        long = "path",
        value_name = "path",
        about = "Library path to load lints from"
    )]
    pub paths: Vec<String>,

    #[clap(
        short,
        long,
        about = "Do not show warnings or progress running commands besides `cargo check`"
    )]
    pub quiet: bool,

    #[clap(long, about = "Check all packages in the workspace")]
    pub workspace: bool,

    #[clap(
        about = "Libraries to load lints from. Each <name> is searched for as described under \
        `--lib`. If no library is found, <name> is treated as path. To avoid ambiguity, use \
        `--lib` or `--path`."
    )]
    pub names: Vec<String>,

    #[clap(last = true, about = "Arguments for `cargo check`")]
    pub args: Vec<String>,
}

impl From<Dylint> for dylint::Dylint {
    fn from(opts: Dylint) -> Self {
        let Dylint {
            all,
            libs,
            list,
            manifest_path,
            no_build,
            no_metadata,
            packages,
            paths,
            quiet,
            workspace,
            names,
            args,
        } = opts;
        Self {
            all,
            libs,
            list,
            manifest_path,
            no_build,
            no_metadata,
            packages,
            paths,
            quiet,
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
