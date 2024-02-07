use clap::{crate_version, ArgAction, Parser};
use std::{
    ffi::{OsStr, OsString},
    fmt::Debug,
};

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
// smoelius: Please keep the field `name_opts` first, and the fields `subcmd`, `names`, and `args`
// last. Please keep all other fields sorted.
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

    #[clap(long = "new", hide = true)]
    new_path: Option<String>,

    #[clap(long, help = "Do not check other packages within the workspace")]
    no_deps: bool,

    #[clap(
        action = ArgAction::Append,
        number_of_values = 1,
        short,
        long = "package",
        value_name = "spec",
        help = "Package to check"
    )]
    packages: Vec<String>,

    #[clap(long, value_name = "path", help = "Path to pipe stderr to")]
    pipe_stderr: Option<String>,

    #[clap(long, value_name = "path", help = "Path to pipe stdout to")]
    pipe_stdout: Option<String>,

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
    subcmd: Option<DylintSubcommand>,

    #[clap(hide = true)]
    names: Vec<String>,

    #[clap(last = true, help = "Arguments for `cargo check`")]
    args: Vec<String>,
}

#[derive(Debug, Parser)]
enum DylintSubcommand {
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
        long_about = "Upgrade the library package at <PATH> to the latest version of \
                      `clippy_utils`"
    )]
    Upgrade {
        #[clap(long, hide = true)]
        allow_downgrade: bool,

        #[clap(
            long,
            help = "Unix only/experimental: Update dependencies and search for the most recent \
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
        long,
        requires("git"),
        help = "Branch to use when downloading library packages"
    )]
    branch: Option<String>,

    #[clap(
        long,
        value_name = "url",
        conflicts_with("paths"),
        help = "Git url containing library packages"
    )]
    git: Option<String>,

    #[clap(
        action = ArgAction::Append,
        number_of_values = 1,
        long = "lib-path",
        value_name = "path",
        help = "Library path to load lints from"
    )]
    lib_paths: Vec<String>,

    #[clap(
        action = ArgAction::Append,
        number_of_values = 1,
        long = "lib",
        value_name = "name",
        help = "Library name to load lints from. A file with a name of the form \"DLL_PREFIX \
        <name> '@' TOOLCHAIN DLL_SUFFIX\" is searched for in the directories listed in \
        DYLINT_LIBRARY_PATH, and in the `target/release` directories produced by building the \
        current workspace's metadata entries (see example below)."
    )]
    libs: Vec<String>,

    #[clap(
        long,
        value_name = "path",
        help = "Path to Cargo.toml. Note: if the manifest uses metadata, then `--manifest-path \
                <path>` must appear before `--`, not after."
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
        value_name = "path",
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

#[allow(deprecated)]
impl From<Dylint> for dylint::opts::Dylint {
    fn from(opts: Dylint) -> Self {
        let opts = process_deprecated_options(opts);
        let Dylint {
            name_opts:
                NameOpts {
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
                },
            allow_downgrade,
            bisect,
            fix,
            force,
            isolate,
            keep_going,
            list,
            new_path,
            no_deps,
            packages,
            pipe_stderr,
            pipe_stdout,
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
            branch,
            fix,
            force,
            git,
            isolate,
            keep_going,
            lib_paths,
            libs,
            list,
            manifest_path,
            new_path,
            no_build,
            no_deps,
            no_metadata,
            packages,
            paths,
            pattern,
            pipe_stderr,
            pipe_stdout,
            quiet,
            rev,
            rust_version,
            tag,
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
            &dylint::opts::Dylint::default(),
            "`--list` is deprecated. Use subcommand `list`.",
        );
    }
    if opts.new_path.is_some() {
        dylint::__warn(
            &dylint::opts::Dylint::default(),
            "`--new` is deprecated. Use subcommand `new`.",
        );
    }
    if opts.upgrade_path.is_some() {
        dylint::__warn(
            &dylint::opts::Dylint::default(),
            "`--upgrade` is deprecated. Use subcommand `upgrade`.",
        );
    }
    if !opts.names.is_empty() {
        dylint::__warn(
            &dylint::opts::Dylint::default(),
            "Referring to libraries by bare name is deprecated. Use `--lib` or `--lib-path`.",
        );
    }
    if let Some(subcmd) = opts.subcmd.take() {
        match subcmd {
            DylintSubcommand::List { name_opts } => {
                opts.name_opts.absorb(name_opts);
                opts.list = true;
            }
            DylintSubcommand::New { isolate, path } => {
                opts.isolate |= isolate;
                opts.new_path = Some(path);
            }
            DylintSubcommand::Upgrade {
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

impl NameOpts {
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

fn main() -> dylint::ColorizedResult<()> {
    env_logger::try_init().unwrap_or_else(|error| {
        dylint::__warn(
            &dylint::opts::Dylint::default(),
            &format!("`env_logger` already initialized: {error}"),
        );
    });

    let args: Vec<_> = std::env::args().map(OsString::from).collect();

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
    use assert_cmd::prelude::*;
    use clap::CommandFactory;
    use predicates::prelude::*;

    #[test]
    fn verify_cli() {
        Opts::command().debug_assert();
    }

    #[test]
    fn usage() {
        std::process::Command::cargo_bin("cargo-dylint")
            .unwrap()
            .args(["dylint", "--help"])
            .assert()
            .success()
            .stdout(predicates::str::contains("Usage: cargo dylint"));
    }

    #[test]
    fn version() {
        std::process::Command::cargo_bin("cargo-dylint")
            .unwrap()
            .args(["dylint", "--version"])
            .assert()
            .success()
            .stdout(format!("cargo-dylint {}\n", env!("CARGO_PKG_VERSION")));
    }

    /// `no_env_logger_warning` fails if [`std::process::Command::new`] is replaced with
    /// [`assert_cmd::cargo::CommandCargoExt::cargo_bin`]. I don't understand why.
    ///
    /// [`assert_cmd::cargo::CommandCargoExt::cargo_bin`]: https://docs.rs/assert_cmd/latest/assert_cmd/cargo/trait.CommandCargoExt.html#tymethod.cargo_bin
    /// [`std::process::Command::new`]: https://doc.rust-lang.org/std/process/struct.Command.html#method.new
    #[test]
    fn no_env_logger_warning() {
        std::process::Command::new("cargo")
            .args(["run", "--bin", "cargo-dylint"])
            // std::process::Command::cargo_bin("cargo-dylint").unwrap()
            .assert()
            .failure()
            .stderr(predicates::str::contains("`env_logger` already initialized").not());
    }
}
