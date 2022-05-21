#![deny(clippy::expect_used)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]

use anyhow::{anyhow, bail, ensure, Context, Result};
use cargo_metadata::MetadataCommand;
use dylint_internal::{
    env::{self, var},
    parse_filename,
};
use lazy_static::lazy_static;
use std::{
    env::consts,
    ffi::OsStr,
    fmt::Debug,
    path::{Path, PathBuf},
};

#[cfg(feature = "metadata")]
pub(crate) use cargo::{core, sources, util};

pub mod driver_builder;

mod error;
use error::warn;
pub use error::{ColorizedError, ColorizedResult};

mod name_toolchain_map;
pub use name_toolchain_map::{Lazy as NameToolchainMap, ToolchainMap};

#[cfg(feature = "metadata")]
pub(crate) mod metadata;

#[cfg(feature = "metadata")]
mod toml;

#[cfg(feature = "package_options")]
mod package_options;

lazy_static! {
    static ref REQUIRED_FORM: String = format!(
        r#""{}" LIBRARY_NAME "@" TOOLCHAIN "{}""#,
        consts::DLL_PREFIX,
        consts::DLL_SUFFIX
    );
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default)]
pub struct Dylint {
    pub all: bool,
    pub bisect: bool,
    pub fix: bool,
    pub force: bool,
    pub isolate: bool,
    pub keep_going: bool,
    pub libs: Vec<String>,
    pub list: bool,
    pub manifest_path: Option<String>,
    pub new_path: Option<String>,
    pub no_build: bool,
    pub no_metadata: bool,
    pub packages: Vec<String>,
    pub paths: Vec<String>,
    pub quiet: bool,
    pub rust_version: Option<String>,
    pub upgrade_path: Option<String>,
    pub workspace: bool,
    pub names: Vec<String>,
    pub args: Vec<String>,
}

pub fn run(opts: &Dylint) -> Result<()> {
    if opts.bisect {
        #[cfg(not(unix))]
        bail!("`--bisect` is supported only on Unix platforms");

        #[cfg(unix)]
        warn(opts, "`--bisect` is experimental");
    }

    if opts.bisect && opts.upgrade_path.is_none() {
        bail!("`--bisect` can be used only with `--upgrade`");
    }

    if opts.force && opts.upgrade_path.is_none() {
        bail!("`--force` can be used only with `--upgrade`");
    }

    if opts.isolate && opts.new_path.is_none() {
        bail!("`--isolate` can be used only with `--new`");
    }

    if opts.rust_version.is_some() && opts.upgrade_path.is_none() {
        bail!("`--rust-version` can be used only with `--upgrade`");
    }

    #[cfg(feature = "package_options")]
    if let Some(path) = &opts.new_path {
        return package_options::new_package(opts, Path::new(path));
    }

    #[cfg(feature = "package_options")]
    if let Some(path) = &opts.upgrade_path {
        return package_options::upgrade_package(opts, Path::new(path));
    }

    let name_toolchain_map = NameToolchainMap::new(opts);

    run_with_name_toolchain_map(opts, &name_toolchain_map)
}

fn run_with_name_toolchain_map(opts: &Dylint, name_toolchain_map: &NameToolchainMap) -> Result<()> {
    if opts.libs.is_empty() && opts.paths.is_empty() && opts.names.is_empty() && !opts.all {
        if opts.list {
            warn_if_empty(opts, name_toolchain_map)?;
            return list_libs(name_toolchain_map);
        }

        warn(opts, "Nothing to do. Did you forget `--all`?");
        return Ok(());
    }

    let resolved = resolve(opts, name_toolchain_map)?;

    if resolved.is_empty() {
        assert!(opts.libs.is_empty());
        assert!(opts.paths.is_empty());
        assert!(opts.names.is_empty());

        let name_toolchain_map_is_empty = warn_if_empty(opts, name_toolchain_map)?;

        // smoelius: If `name_toolchain_map` is NOT empty, then it had better be the case that
        // `--all` was not passed.
        assert!(name_toolchain_map_is_empty || !opts.all);
    }

    if opts.list {
        list_lints(opts, &resolved)
    } else {
        check_or_fix(opts, &resolved)
    }
}

fn warn_if_empty(opts: &Dylint, name_toolchain_map: &NameToolchainMap) -> Result<bool> {
    let name_toolchain_map = name_toolchain_map.get_or_try_init()?;

    Ok(if name_toolchain_map.is_empty() {
        warn(opts, "No libraries were found.");
        true
    } else {
        false
    })
}

fn list_libs(name_toolchain_map: &NameToolchainMap) -> Result<()> {
    let name_toolchain_map = name_toolchain_map.get_or_try_init()?;

    let name_width = name_toolchain_map
        .keys()
        .map(String::len)
        .max()
        .unwrap_or_default();

    let toolchain_width = name_toolchain_map
        .values()
        .flat_map(ToolchainMap::keys)
        .map(String::len)
        .max()
        .unwrap_or_default();

    for (name, toolchain_map) in name_toolchain_map {
        for (toolchain, paths) in toolchain_map {
            for path in paths {
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

#[allow(unknown_lints)]
#[allow(question_mark_in_expression)]
fn resolve(opts: &Dylint, name_toolchain_map: &NameToolchainMap) -> Result<ToolchainMap> {
    let mut toolchain_map = ToolchainMap::new();

    if opts.all {
        let name_toolchain_map = name_toolchain_map.get_or_try_init()?;

        name_toolchain_map.values().cloned().for_each(|other| {
            for (toolchain, mut paths) in other {
                toolchain_map
                    .entry(toolchain)
                    .or_insert_with(Default::default)
                    .append(&mut paths);
            }
        });
    }

    for name in &opts.libs {
        ensure!(!opts.all, "`--lib` cannot be used with `--all`");
        let (toolchain, path) =
            name_as_lib(name_toolchain_map, name, true)?.unwrap_or_else(|| unreachable!());
        toolchain_map
            .entry(toolchain)
            .or_insert_with(Default::default)
            .insert(path);
    }

    for name in &opts.paths {
        let (toolchain, path) = name_as_path(name, true)?.unwrap_or_else(|| unreachable!());
        toolchain_map
            .entry(toolchain)
            .or_insert_with(Default::default)
            .insert(path);
    }

    for name in &opts.names {
        if let Some((toolchain, path)) = name_as_lib(name_toolchain_map, name, false)? {
            ensure!(
                !opts.all,
                "`{}` is a library name and cannot be used with `--all`; if a path was meant, use `--path {}`",
                name,
                name
            );
            toolchain_map
                .entry(toolchain)
                .or_insert_with(Default::default)
                .insert(path);
        } else if let Some((toolchain, path)) = name_as_path(name, false)? {
            toolchain_map
                .entry(toolchain)
                .or_insert_with(Default::default)
                .insert(path);
        } else {
            bail!("Could not find `{}`", name);
        }
    }

    Ok(toolchain_map)
}

pub fn name_as_lib(
    name_toolchain_map: &NameToolchainMap,
    name: &str,
    as_lib_only: bool,
) -> Result<Option<(String, PathBuf)>> {
    if !is_valid_lib_name(name) {
        ensure!(!as_lib_only, "`{}` is not a valid library name", name);
        return Ok(None);
    }

    let name_toolchain_map = name_toolchain_map.get_or_try_init()?;

    if let Some(toolchain_map) = name_toolchain_map.get(name) {
        let mut toolchain_paths = flatten_toolchain_map(toolchain_map);

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

            // smoelius: If `name` contains a path separator, then it was clearly meant to be a
            // path.
            ensure!(
                !name.contains(std::path::MAIN_SEPARATOR),
                "`{}` is a valid path, but the filename does not have the required form: {}",
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

fn list_lints(opts: &Dylint, resolved: &ToolchainMap) -> Result<()> {
    for (toolchain, paths) in resolved {
        for path in paths {
            if resolved
                .get(toolchain)
                .map_or(false, |paths| paths.contains(path))
            {
                let driver = driver_builder::get(opts, toolchain)?;
                let dylint_libs = serde_json::to_string(&[path])?;
                let filename = path
                    .file_name()
                    .ok_or_else(|| anyhow!("Could not get file name"))?;
                let (name, _) = parse_filename(&filename.to_string_lossy())
                    .ok_or_else(|| anyhow!("Could not parse file name"))?;

                print!("{}", name);
                if resolved.keys().len() >= 2 {
                    print!("@{}", toolchain);
                }
                if paths.len() >= 2 {
                    let parent = path
                        .parent()
                        .ok_or_else(|| anyhow!("Could not get parent directory"))?;
                    print!(" ({})", parent.to_string_lossy());
                }
                println!();

                // smoelius: `-W help` is the normal way to list lints, so we can be sure it
                // gets the lints loaded. However, we don't actually use it to list the lints.
                let mut command = dylint_internal::driver(toolchain, &driver)?;
                command
                    .envs(vec![
                        (env::DYLINT_LIBS, dylint_libs.as_str()),
                        (env::DYLINT_LIST, "1"),
                    ])
                    .args(vec!["rustc", "-W", "help"])
                    .success()?;

                println!();
            }
        }
    }

    Ok(())
}

fn check_or_fix(opts: &Dylint, resolved: &ToolchainMap) -> Result<()> {
    let clippy_disable_docs_links = clippy_disable_docs_links()?;

    let mut failures = Vec::new();

    for (toolchain, paths) in resolved {
        let target_dir = target_dir(opts, toolchain)?;
        let target_dir_str = target_dir.to_string_lossy();
        let driver = driver_builder::get(opts, toolchain)?;
        let dylint_libs = serde_json::to_string(&paths)?;
        let description = format!("with toolchain `{}`", toolchain);
        let mut command = if opts.fix {
            dylint_internal::cargo::fix(&description)
        } else {
            dylint_internal::cargo::check(&description)
        };
        let mut args = vec!["--target-dir", &target_dir_str];
        if let Some(path) = &opts.manifest_path {
            args.extend(&["--manifest-path", path]);
        }
        for spec in &opts.packages {
            args.extend(&["-p", spec]);
        }
        if opts.workspace {
            args.extend(&["--workspace"]);
        }
        args.extend(opts.args.iter().map(String::as_str));

        // smoelius: Set CLIPPY_DISABLE_DOCS_LINKS to prevent lints from accidentally linking to the
        // Clippy repository. But set it to the JSON-encoded original value so that the Clippy
        // library can unset the variable.
        // smoelius: This doesn't work if another library is loaded alongside Clippy.
        // smoelius: This was fixed in `clippy_utils`:
        // https://github.com/rust-lang/rust-clippy/commit/1a206fc4abae0b57a3f393481367cf3efca23586
        // But I am going to continue to set CLIPPY_DISABLE_DOCS_LINKS because it doesn't seem to
        // hurt and it provides a small amount of backward compatibility.
        let result = command
            .envs(vec![
                (
                    env::CLIPPY_DISABLE_DOCS_LINKS,
                    clippy_disable_docs_links.as_str(),
                ),
                (env::DYLINT_LIBS, &dylint_libs),
                (env::RUSTC_WORKSPACE_WRAPPER, &*driver.to_string_lossy()),
                (env::RUSTUP_TOOLCHAIN, toolchain),
            ])
            .args(args)
            .success();
        if result.is_err() {
            if !opts.keep_going {
                return result
                    .with_context(|| format!("Compilation failed with toolchain `{}`", toolchain));
            };
            failures.push(toolchain);
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "Compilation failed with the following toolchains: {:?}",
            failures
        ))
    }
}

fn target_dir(opts: &Dylint, toolchain: &str) -> Result<PathBuf> {
    let mut command = MetadataCommand::new();
    if let Some(path) = &opts.manifest_path {
        command.manifest_path(path);
    }
    let metadata = command.no_deps().exec()?;
    Ok(metadata
        .target_directory
        .join("dylint")
        .join("target")
        .join(toolchain)
        .into())
}

fn flatten_toolchain_map(toolchain_map: &ToolchainMap) -> Vec<(String, PathBuf)> {
    toolchain_map
        .iter()
        .flat_map(|(toolchain, paths)| {
            paths
                .iter()
                .map(|path| (toolchain.clone(), path.clone()))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn clippy_disable_docs_links() -> Result<String> {
    let val = var(env::CLIPPY_DISABLE_DOCS_LINKS).ok();
    serde_json::to_string(&val).map_err(Into::into)
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use dylint_internal::{cargo::current_metadata, examples};
    use lazy_static::lazy_static;
    use std::env::{join_paths, set_var};
    use test_log::test;

    lazy_static! {
        static ref OPTS: Dylint = Dylint {
                no_metadata: true,
                ..Dylint::default()
            };
        static ref NAME_TOOLCHAIN_MAP: NameToolchainMap<'static> = {
            examples::build().unwrap();
            let metadata = current_metadata().unwrap();
            // smoelius: As of version 0.1.14, `cargo-llvm-cov` no longer sets `CARGO_TARGET_DIR`.
            // So `dylint_library_path` no longer requires a `cfg!(coverage)` special case.
            let dylint_library_path = join_paths(&[
                    metadata.target_directory.join("examples").join("debug"),
                    metadata.target_directory.join("straggler").join("debug"),
                ])
                .unwrap();
            set_var(env::DYLINT_LIBRARY_PATH, dylint_library_path);
            NameToolchainMap::new(&OPTS)
        };
    }

    #[allow(unknown_lints)]
    #[allow(non_thread_safe_call_in_test)]
    #[test]
    fn multiple_libraries_multiple_toolchains() {
        let name_toolchain_map = NAME_TOOLCHAIN_MAP.get_or_try_init().unwrap();

        let question_mark_in_expression = name_toolchain_map
            .get("question_mark_in_expression")
            .unwrap();
        let straggler = name_toolchain_map.get("straggler").unwrap();

        assert_ne!(
            question_mark_in_expression.keys().collect::<Vec<_>>(),
            straggler.keys().collect::<Vec<_>>()
        );

        let opts = Dylint {
            libs: vec![
                "question_mark_in_expression".to_owned(),
                "straggler".to_owned(),
            ],
            ..Dylint::default()
        };

        run_with_name_toolchain_map(&opts, &NAME_TOOLCHAIN_MAP).unwrap();
    }

    // smoelius: Check that loading multiple libraries with the same Rust toolchain works. At one
    // point, I was getting this error from `libloading`:
    //
    //   cannot allocate memory in static TLS block
    //
    // The culprit turned out to be the `rand` crate, which uses a lot of thread local storage.
    // `rand` is used by `tempfile`, which is used by various Rust compiler crates. Essentially,
    // each library had its own copy of the Rust compiler, and therefore its own copy of the `rand`
    // crate, and this was eating up all the thread local storage.
    //
    // The solution was to add `extern crate rustc_driver` to each library. This causes the library
    // to link against `librust_driver.so`, which dylint-driver also links against. So, essentially,
    // each library now uses dylint-driver's copy of the `rand` crate.
    //
    // This thread was very helpful in diagnosing the problem:
    //
    //   https://bugzilla.redhat.com/show_bug.cgi?id=1722181
    //
    #[allow(unknown_lints)]
    #[allow(non_thread_safe_call_in_test)]
    #[test]
    fn multiple_libraries_one_toolchain() {
        let name_toolchain_map = NAME_TOOLCHAIN_MAP.get_or_try_init().unwrap();

        let clippy = name_toolchain_map.get("clippy").unwrap();
        let question_mark_in_expression = name_toolchain_map
            .get("question_mark_in_expression")
            .unwrap();

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

        run_with_name_toolchain_map(&opts, &NAME_TOOLCHAIN_MAP).unwrap();
    }
}
