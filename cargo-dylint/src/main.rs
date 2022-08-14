use clap::{crate_version, Parser};
use dylint_internal::env;
use std::{
    ffi::{OsStr, OsString},
    fmt::Debug,
};

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo")]
struct Opts {
    #[clap(subcommand)]
    subcmd: CargoSubCommand,
}

#[derive(Debug, Parser)]
enum CargoSubCommand {
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

DYLINT_RUSTFLAGS (default: none) is a space-separated list of flags that Dylint passes to `rustc`.

METADATA EXAMPLE:

    [workspace.metadata.dylint]
    libraries = [
        { git = "https://github.com/trailofbits/dylint", pattern = "examples/*/*" },
        { path = "libs/*" },
    ]
"#,
)]
struct Dylint {
    #[clap(flatten)]
    name_opts: NameOpts,

    #[clap(long, hide = true)]
    allow_downgrade: bool,

    #[clap(long, hide = true)]
    bisect: bool,

    #[clap(long, help = "Automatically apply lint suggestions")]
    fix: bool,

    #[clap(long, hide = true)]
    force: bool,

    #[clap(long, hide = true)]
    isolate: bool,

    #[clap(long, help = "Continue if `cargo check` fails")]
    keep_going: bool,

    #[clap(long, hide = true)]
    list: bool,

    #[clap(
        long,
        value_name = "path",
        help = "Path to Cargo.toml. Note: if the manifest uses metadata, then \
        `--manifest-path <path>` must appear before `--`, not after."
    )]
    manifest_path: Option<String>,

    #[clap(long = "new", hide = true)]
    new_path: Option<String>,

    #[clap(
        multiple_occurrences = true,
        number_of_values = 1,
        short,
        long = "package",
        value_name = "spec",
        help = "Package to check"
    )]
    packages: Vec<String>,

    #[clap(
        global = true,
        short,
        long,
        help = "Do not show warnings or progress running commands besides `cargo check` and \
        `cargo fix`"
    )]
    quiet: bool,

    #[clap(long, hide = true)]
    rust_version: Option<String>,

    #[clap(long = "upgrade", hide = true)]
    upgrade_path: Option<String>,

    #[clap(long, help = "Check all packages in the workspace")]
    workspace: bool,

    #[clap(subcommand)]
    subcmd: Option<DylintSubCommand>,

    #[clap(hide = true)]
    names: Vec<String>,

    #[clap(last = true, help = "Arguments for `cargo check`")]
    args: Vec<String>,
}

#[derive(Debug, Parser)]
enum DylintSubCommand {
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
        name_opts: NameOpts,
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
        long_about = "Upgrade the library package at <PATH> to the latest version of `clippy_utils`"
    )]
    Upgrade {
        #[clap(long, hide = true)]
        allow_downgrade: bool,

        #[clap(
            long,
            help = "Unix only/experimental: Update dependencies and search for the earliest \
            applicable toolchain"
        )]
        bisect: bool,

        #[clap(
            long,
            value_name = "version",
            help = "Upgrade to the version of `clippy_utils` with tag `rust-<version>`"
        )]
        rust_version: Option<String>,

        #[clap(help = "Path to library package")]
        path: String,
    },
}

#[derive(Debug, Parser)]
struct NameOpts {
    #[clap(long, help = "Load all discovered libraries")]
    all: bool,

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
    libs: Vec<String>,

    #[clap(long, help = "Do not build metadata entries")]
    no_build: bool,

    #[clap(long, help = "Ignore metadata entirely")]
    no_metadata: bool,

    #[clap(
        multiple_occurrences = true,
        number_of_values = 1,
        long = "path",
        value_name = "path",
        help = "Library path to load lints from"
    )]
    paths: Vec<String>,
}

#[allow(deprecated)]
impl From<Dylint> for dylint::Dylint {
    fn from(opts: Dylint) -> Self {
        let opts = process_deprecated_options(opts);
        let Dylint {
            name_opts:
                NameOpts {
                    all,
                    libs,
                    no_build,
                    no_metadata,
                    paths,
                },
            allow_downgrade,
            bisect,
            fix,
            force,
            isolate,
            keep_going,
            list,
            manifest_path,
            new_path,
            packages,
            quiet,
            rust_version,
            upgrade_path,
            workspace,
            subcmd: _,
            names,
            args,
        } = opts;
        Self {
            all,
            allow_downgrade,
            bisect,
            fix,
            force,
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

fn process_deprecated_options(mut opts: Dylint) -> Dylint {
    if opts.list {
        dylint::__warn(
            &dylint::Dylint::default(),
            "`--list` is deprecated. Use subcommand `list`.",
        );
    }
    if opts.new_path.is_some() {
        dylint::__warn(
            &dylint::Dylint::default(),
            "`--new` is deprecated. Use subcommand `new`.",
        );
    }
    if opts.upgrade_path.is_some() {
        dylint::__warn(
            &dylint::Dylint::default(),
            "`--upgrade` is deprecated. Use subcommand `upgrade`.",
        );
    }
    if !opts.names.is_empty() {
        dylint::__warn(
            &dylint::Dylint::default(),
            "Referring to libraries by bare name is deprecated. Use `--lib` or `--path`.",
        );
    }
    if let Some(subcmd) = opts.subcmd.take() {
        match subcmd {
            DylintSubCommand::List { name_opts } => {
                opts.name_opts.absorb(name_opts);
                opts.list = true;
            }
            DylintSubCommand::New { isolate, path } => {
                opts.isolate |= isolate;
                opts.new_path = Some(path);
            }
            DylintSubCommand::Upgrade {
                allow_downgrade,
                bisect,
                rust_version,
                path,
            } => {
                opts.allow_downgrade |= allow_downgrade;
                opts.bisect |= bisect;
                opts.rust_version = rust_version;
                opts.upgrade_path = Some(path);
            }
        }
    }
    opts
}

impl NameOpts {
    pub fn absorb(&mut self, other: Self) {
        self.all |= other.all;
        self.libs.extend(other.libs);
        self.no_build |= other.no_build;
        self.no_metadata |= other.no_metadata;
        self.paths.extend(other.paths);
    }
}

fn main() -> dylint::ColorizedResult<()> {
    env_logger::init();

    let args: Vec<_> = std::env::args().map(OsString::from).collect();

    let result = cargo_dylint(&args);

    if result.is_err() && env::enabled(env::RUST_BACKTRACE) {
        eprintln!(
            "If you don't see a backtrace below, it could be because `cargo-dylint` wasn't built \
            with a nightly compiler."
        );
    }

    result
}

fn cargo_dylint<T: AsRef<OsStr>>(args: &[T]) -> dylint::ColorizedResult<()> {
    match Opts::parse_from(args).subcmd {
        CargoSubCommand::Dylint(opts) => dylint::run(&dylint::Dylint::from(opts)),
    }
    .map_err(dylint::ColorizedError::new)
}
