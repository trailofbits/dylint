#![deny(clippy::expect_used)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]

use anyhow::{anyhow, bail, ensure, Result};
use cargo_metadata::MetadataCommand;
use clap::{crate_version, lazy_static::lazy_static, Clap};
use dylint_internal::env::{self, var};
use std::{
    collections::{BTreeMap, BTreeSet},
    env::consts,
    ffi::OsStr,
    fmt::Debug,
    fs::read_dir,
    path::{Path, PathBuf},
    process::Command,
};

pub mod driver_builder;

lazy_static! {
    static ref REQUIRED_FORM: String = format!(
        r#""{}" LIBRARY_NAME "@" TOOLCHAIN "{}""#,
        consts::DLL_PREFIX,
        consts::DLL_SUFFIX
    );
}

type ToolchainPathsMap = BTreeMap<String, BTreeSet<PathBuf>>;

pub type NameToolchainPathsMap = BTreeMap<String, ToolchainPathsMap>;

#[derive(Clap, Debug)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap, Debug)]
enum SubCommand {
    Dylint(Dylint),
}

#[derive(Clap, Debug, Default)]
#[clap(
    version = crate_version!(),
    after_help = "ENVIRONMENT VARIABLES:

DYLINT_DRIVER_PATH (default: $HOME/.dylint_drivers) is the directory where Dylint stores rustc
drivers.

DYLINT_LIBRARY_PATH (default: none) is a colon-separated list of directories where Dylint searches
for libraries.
",
)]
pub struct Dylint {
    #[clap(
        multiple = true,
        number_of_values = 1,
        long = "lib",
        value_name = "name",
        about = "Library name to load lints from. A file with a name of the form \"DLL_PREFIX \
        <name> '@' TOOLCHAIN DLL_SUFFIX\" is searched for in the directories listed in \
        DYLINT_LIBRARY_PATH and in the current package's `debug` and `release` directories."
    )]
    pub libs: Vec<String>,

    #[clap(
        long = "list",
        about = "List the name, toolchain, and location of discovered libraries"
    )]
    pub list: bool,

    #[clap(
        multiple = true,
        number_of_values = 1,
        long = "path",
        value_name = "path",
        about = "Library path to load lints from"
    )]
    pub paths: Vec<String>,

    #[clap(
        about = "Libraries to load lints from. Each <name> is searched for as described under \
        `--lib`. If no library is found, <name> is treated as path. To avoid ambiguity, use \
        `--lib` or `--path`."
    )]
    pub names: Vec<String>,

    #[clap(last = true, about = "Arguments for `cargo check`")]
    pub args: Vec<String>,
}

pub fn cargo_dylint<T: AsRef<OsStr>>(args: &[T]) -> Result<()> {
    match Opts::parse_from(args).subcmd {
        SubCommand::Dylint(opts) => run(&opts),
    }
}

pub fn run(opts: &Dylint) -> Result<()> {
    let inventory = inventory()?;

    run_with_inventory(opts, &inventory)
}

pub fn inventory() -> Result<NameToolchainPathsMap> {
    let mut map = NameToolchainPathsMap::new();

    let dylint_library_paths = dylint_library_paths()?;
    let profile_paths = profile_paths();

    for path in dylint_library_paths.iter().chain(&profile_paths) {
        for entry in dylint_libraries_in(path)? {
            let (name, toolchain, path) = entry?;
            map.entry(name)
                .or_insert_with(Default::default)
                .entry(toolchain)
                .or_insert_with(Default::default)
                .insert(path);
        }
    }

    Ok(map)
}

fn dylint_library_paths() -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    if let Ok(val) = var(env::DYLINT_LIBRARY_PATH) {
        for path in val.split(':') {
            let path = PathBuf::from(path);
            ensure!(
                path.is_absolute(),
                "DYLINT_LIBRARY_PATH contains `{}`, which is not absolute",
                path.to_string_lossy()
            );
            ensure!(
                path.is_dir(),
                "DYLINT_LIBRARY_PATH contains `{}`, which is not a directory",
                path.to_string_lossy()
            );
            paths.push(path);
        }
    }

    Ok(paths)
}

fn profile_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(metadata) = MetadataCommand::new().no_deps().exec() {
        let debug = metadata.target_directory.join_os("debug");
        let release = metadata.target_directory.join_os("release");
        if debug.is_dir() {
            paths.push(debug);
        }
        if release.is_dir() {
            paths.push(release);
        }
    }

    paths
}

#[allow(clippy::option_if_let_else)]
fn dylint_libraries_in(
    path: &Path,
) -> Result<impl Iterator<Item = Result<(String, String, PathBuf)>>> {
    let iter = read_dir(path)?;
    Ok(iter
        .map(|entry| -> Result<Option<(String, String, PathBuf)>> {
            let entry = entry?;
            let path = entry.path();

            Ok(if let Some(filename) = path.file_name() {
                parse_filename(&filename.to_string_lossy())
                    .map(|(lib_name, toolchain)| (lib_name, toolchain, path))
            } else {
                None
            })
        })
        .filter_map(Result::transpose))
}

fn run_with_inventory(opts: &Dylint, inventory: &NameToolchainPathsMap) -> Result<()> {
    if opts.list {
        return list(inventory);
    }

    let clippy_disable_docs_links = clippy_disable_docs_links()?;

    let toolchain_paths_map = toolchain_paths_map(opts, inventory)?;

    for (toolchain, paths) in toolchain_paths_map {
        let driver = driver_builder::get(&toolchain)?;
        let dylint_libs = serde_json::to_string(&paths)?;

        let mut command = Command::new("cargo");

        // smoelius: Set CLIPPY_DISABLE_DOCS_LINKS to prevent lints from accidentally linking to the
        // Clippy repository. But set it to the JSON-encoded original value so that the Clippy
        // library can unset the variable.
        // smoelius: This doesn't work if another library is loaded alongside Clippy.
        // smoelius: This was fixed in `clippy_utils`:
        // https://github.com/rust-lang/rust-clippy/commit/1a206fc4abae0b57a3f393481367cf3efca23586
        // But I am going to continue to set CLIPPY_DISABLE_DOCS_LINKS because it doesn't seem to
        // hurt and it provides a small amount of backward compatibility.
        let envs = vec![
            (
                env::CLIPPY_DISABLE_DOCS_LINKS.to_owned(),
                clippy_disable_docs_links.clone(),
            ),
            (env::DYLINT_LIBS.to_owned(), dylint_libs),
            (
                env::RUSTC_WORKSPACE_WRAPPER.to_owned(),
                driver.to_string_lossy().to_string(),
            ),
            (env::RUSTUP_TOOLCHAIN.to_owned(), toolchain),
        ];

        let mut args = vec!["check".to_owned()];
        args.extend(opts.args.clone());

        log::debug!("{:?}", envs);

        command.envs(envs).args(args);

        log::debug!("{:?}", command);

        let status = command.status()?;

        ensure!(status.success(), "command failed: {:?}", command);
    }

    Ok(())
}

fn list(inventory: &NameToolchainPathsMap) -> Result<()> {
    let name_width = inventory.keys().map(String::len).max().unwrap_or_default();
    let toolchain_width = inventory
        .values()
        .flat_map(ToolchainPathsMap::keys)
        .map(String::len)
        .max()
        .unwrap_or_default();

    for (name, map) in inventory {
        for (toolchain, set) in map {
            for path in set {
                let parent = path
                    .parent()
                    .ok_or_else(|| anyhow!("Could not get parent directory"))?;
                println!(
                    "{:<name_width$} {:<toolchain_width$} {}",
                    name,
                    toolchain,
                    parent.to_string_lossy(),
                    name_width = name_width,
                    toolchain_width = toolchain_width
                );
            }
        }
    }

    Ok(())
}

fn clippy_disable_docs_links() -> Result<String> {
    let val = var(env::CLIPPY_DISABLE_DOCS_LINKS).ok();
    serde_json::to_string(&val).map_err(Into::into)
}

#[allow(unknown_lints)]
#[allow(question_mark_in_expression)]
fn toolchain_paths_map(
    opts: &Dylint,
    inventory: &NameToolchainPathsMap,
) -> Result<ToolchainPathsMap> {
    let mut map = ToolchainPathsMap::new();

    for name in &opts.libs {
        let (toolchain, path) =
            name_as_lib(inventory, name, true)?.unwrap_or_else(|| unreachable!());
        map.entry(toolchain)
            .or_insert_with(Default::default)
            .insert(path);
    }

    for name in &opts.paths {
        let (toolchain, path) = name_as_path(name, true)?.unwrap_or_else(|| unreachable!());
        map.entry(toolchain)
            .or_insert_with(Default::default)
            .insert(path);
    }

    for name in &opts.names {
        if let Some((toolchain, path)) = name_as_lib(inventory, name, false)? {
            map.entry(toolchain)
                .or_insert_with(Default::default)
                .insert(path);
        } else if let Some((toolchain, path)) = name_as_path(name, false)? {
            map.entry(toolchain)
                .or_insert_with(Default::default)
                .insert(path);
        } else {
            bail!("Could not find `{}`", name);
        }
    }

    Ok(map)
}

pub fn name_as_lib(
    inventory: &NameToolchainPathsMap,
    name: &str,
    as_lib_only: bool,
) -> Result<Option<(String, PathBuf)>> {
    if !is_valid_lib_name(name) {
        ensure!(!as_lib_only, "`{}` is not a valid library name", name);
        return Ok(None);
    }

    if let Some(map) = inventory.get(name) {
        let mut toolchain_paths: Vec<(String, PathBuf)> = map
            .iter()
            .flat_map(|(toolchain, paths)| {
                paths
                    .iter()
                    .map(|path| (toolchain.clone(), path.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();

        return match toolchain_paths.len() {
            0 => Ok(None),
            1 => Ok(Some(toolchain_paths.remove(0))),
            _ => Err(anyhow!(
                "Found multiple libraries matching `{}`: {:?}",
                name,
                toolchain_paths
                    .iter()
                    .map(|(_, path)| path)
                    .collect::<Vec<_>>()
            )),
        };
    }

    ensure!(!as_lib_only, "Could not find `--lib {}`", name);

    Ok(None)
}

fn is_valid_lib_name(name: &str) -> bool {
    Path::new(name).file_name() == Some(OsStr::new(name))
}

fn name_as_path(name: &str, as_path_only: bool) -> Result<Option<(String, PathBuf)>> {
    if let Ok(path) = PathBuf::from(name).canonicalize() {
        if let Some(filename) = path.file_name() {
            if let Some((_, toolchain)) = parse_filename(&filename.to_string_lossy()) {
                return Ok(Some((toolchain, path)));
            }

            ensure!(
                !as_path_only,
                "`--path {}` was used, but the filename does not have the required form: {}",
                name,
                *REQUIRED_FORM
            );

            ensure!(
                !name.contains(std::path::MAIN_SEPARATOR),
                "`{}` looks like a path, but the filename does not have the required form: {}",
                name,
                *REQUIRED_FORM
            );
        }

        ensure!(
            !as_path_only,
            "`--path {}` was used, but it is invalid",
            name
        );
    }

    ensure!(!as_path_only, "Could not find `--path {}`", name);

    Ok(None)
}

fn parse_filename(filename: &str) -> Option<(String, String)> {
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

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use dylint_examples as examples;
    use lazy_static::lazy_static;
    use std::env::set_var;

    lazy_static! {
        static ref INVENTORY: NameToolchainPathsMap = {
            examples::build().unwrap();
            let dylint_library_path = examples::iter()
                .unwrap()
                .map(|example| {
                    example
                        .unwrap()
                        .join("target")
                        .join("debug")
                        .to_string_lossy()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(":");
            set_var(env::DYLINT_LIBRARY_PATH, dylint_library_path);
            inventory().unwrap()
        };
    }

    #[test]
    fn multiple_libraries_multiple_toolchains() {
        let allow_clippy = INVENTORY.get("allow_clippy").unwrap();
        let question_mark_in_expression = INVENTORY.get("question_mark_in_expression").unwrap();

        assert_ne!(
            allow_clippy.keys().collect::<Vec<_>>(),
            question_mark_in_expression.keys().collect::<Vec<_>>()
        );

        let opts = Dylint {
            libs: vec![
                "allow_clippy".to_owned(),
                "question_mark_in_expression".to_owned(),
            ],
            ..Dylint::default()
        };

        run_with_inventory(&opts, &INVENTORY).unwrap();
    }

    // smoelius: Check that loading multiple libraries with the same Rust toolchain works. At one point,
    // I was getting this error from `libloading`:
    //
    //   cannot allocate memory in static TLS block
    //
    // The culprit turned out to be the `rand` crate, which uses a lot of thread local storage. `rand`
    // is used by `tempfile`, which is used by various Rust compiler crates. Essentially, each library
    // had its own copy of the Rust compiler, and therefore its own copy of the `rand` crate, and this
    // was eating up all the thread local storage.
    //
    // The solution was to add `extern crate rustc_driver` to each library. This causes the library to
    // link against `librust_driver.so`, which dylint-driver also links against. So, essentially, each
    // library now uses dylint-driver's copy of the `rand` crate.
    //
    // This thread was very helpful in diagnosing the problem:
    //
    //   https://bugzilla.redhat.com/show_bug.cgi?id=1722181
    //
    #[test]
    fn multiple_libraries_one_toolchain() {
        let clippy = INVENTORY.get("clippy").unwrap();
        let question_mark_in_expression = INVENTORY.get("question_mark_in_expression").unwrap();

        assert_eq!(
            clippy.keys().collect::<Vec<_>>(),
            question_mark_in_expression.keys().collect::<Vec<_>>()
        );

        let opts = Dylint {
            libs: vec![
                "clippy".to_owned(),
                "question_mark_in_expression".to_owned(),
            ],
            ..Dylint::default()
        };

        run_with_inventory(&opts, &INVENTORY).unwrap();
    }
}
