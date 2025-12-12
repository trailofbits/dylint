#![cfg_attr(dylint_lib = "general", allow(crate_wide_allow))]
#![cfg_attr(dylint_lib = "supplementary", allow(nonexistent_path_in_comment))]
#![deny(clippy::expect_used)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]

use anyhow::{Context, Result, anyhow, bail, ensure};
use cargo_metadata::MetadataCommand;
use dylint_internal::{
    CommandExt, driver as dylint_driver, env, parse_path_filename, rustup::SanitizeEnvironment,
};
use std::{
    collections::BTreeMap,
    env::{consts, current_dir},
    ffi::OsStr,
    fs::OpenOptions,
    path::{MAIN_SEPARATOR, Path, PathBuf},
    sync::LazyLock,
};

type Object = serde_json::Map<String, serde_json::Value>;

pub mod driver_builder;

mod error;
use error::warn;
#[doc(hidden)]
pub use error::warn as __warn;
pub use error::{ColorizedError, ColorizedResult};

mod name_toolchain_map;
pub use name_toolchain_map::{Lazy as NameToolchainMap, ToolchainMap};
use name_toolchain_map::{LazyToolchainMap, MaybeLibrary};

#[cfg(__library_packages)]
pub(crate) mod library_packages;

pub mod opts;

#[cfg(feature = "package_options")]
mod package_options;

static REQUIRED_FORM: LazyLock<String> = LazyLock::new(|| {
    format!(
        r#""{}" LIBRARY_NAME "@" TOOLCHAIN "{}""#,
        consts::DLL_PREFIX,
        consts::DLL_SUFFIX
    )
});

#[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_error_return))]
pub fn run(opts: &opts::Dylint) -> Result<()> {
    let opts = {
        let mut opts = opts.clone();

        if matches!(
            opts.operation,
            opts::Operation::Check(_) | opts::Operation::List(_)
        ) {
            let lib_sel = opts.library_selection_mut();

            // smoelius: Use of `--git` or `--path` implies `--all`.
            //
            // smoelius: The intuition is as follows. Option `--git` or `--path` creates an
            // alternative namespace of library packages, i.e., alternative to the one built
            // from workspace metadata. The alternative namespace consists of just the
            // library packages in the `--git`-named repository, or the `--path`-named
            // directory.
            //
            // It would seem silly to require the user to pass `--lib` in addition to `--git` or
            // `--path`. For this reason, all of the libraries in the alternative namespace are
            // loaded.
            lib_sel.all |= lib_sel.git_or_path();
        }

        opts
    };

    if opts.has_library_selection()
        && opts.library_selection().pattern.is_some()
        && !opts.git_or_path()
    {
        bail!("`--pattern` can be used only with `--git` or `--path`");
    }

    if opts.pipe_stderr.is_some() {
        warn(&opts, "`--pipe-stderr` is experimental");
    }

    if opts.pipe_stdout.is_some() {
        warn(&opts, "`--pipe-stdout` is experimental");
    }

    match &opts.operation {
        opts::Operation::Check(_) | opts::Operation::List(_) => {
            let name_toolchain_map = NameToolchainMap::new(&opts);
            run_with_name_toolchain_map(&opts, &name_toolchain_map)
        }
        #[cfg(feature = "package_options")]
        opts::Operation::New(new_opts) => package_options::new_package(&opts, new_opts),
        #[cfg(feature = "package_options")]
        opts::Operation::Upgrade(upgrade_opts) => {
            package_options::upgrade_package(&opts, upgrade_opts)
        }
    }
}

fn run_with_name_toolchain_map(
    opts: &opts::Dylint,
    name_toolchain_map: &NameToolchainMap,
) -> Result<()> {
    let lib_sel = opts.library_selection();

    if lib_sel.libs.is_empty() && lib_sel.lib_paths.is_empty() && !lib_sel.all {
        if matches!(opts.operation, opts::Operation::List(_)) {
            warn_if_empty(opts, name_toolchain_map)?;
            return list_libs(name_toolchain_map);
        }

        warn(opts, "Nothing to do. Did you forget `--all`?");
        return Ok(());
    }

    let resolved = resolve(opts, name_toolchain_map)?;

    if resolved.is_empty() {
        assert!(lib_sel.libs.is_empty());
        assert!(lib_sel.lib_paths.is_empty());

        let name_toolchain_map_is_empty = warn_if_empty(opts, name_toolchain_map)?;

        // smoelius: If `name_toolchain_map` is NOT empty, then it had better be the case that
        // `--all` was not passed.
        assert!(name_toolchain_map_is_empty || !lib_sel.all);
    }

    match &opts.operation {
        opts::Operation::Check(check_opts) => check_or_fix(opts, check_opts, &resolved),
        opts::Operation::List(_) => list_lints(opts, &resolved),
        #[allow(unreachable_patterns)]
        _ => unreachable!(),
    }
}

fn warn_if_empty(opts: &opts::Dylint, name_toolchain_map: &NameToolchainMap) -> Result<bool> {
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
        .flat_map(LazyToolchainMap::keys)
        .map(String::len)
        .max()
        .unwrap_or_default();

    for (name, toolchain_map) in name_toolchain_map {
        for (toolchain, maybe_libraries) in toolchain_map {
            for maybe_library in maybe_libraries {
                let location = display_location(&maybe_library.path())?;
                println!("{name:<name_width$}  {toolchain:<toolchain_width$}  {location}",);
            }
        }
    }

    Ok(())
}

#[cfg_attr(
    dylint_lib = "question_mark_in_expression",
    allow(question_mark_in_expression)
)]
fn resolve(opts: &opts::Dylint, name_toolchain_map: &NameToolchainMap) -> Result<ToolchainMap> {
    let lib_sel = opts.library_selection();

    let mut toolchain_map = ToolchainMap::new();

    if lib_sel.all {
        let name_toolchain_map = name_toolchain_map.get_or_try_init()?;

        for other in name_toolchain_map.values() {
            for (toolchain, maybe_libraries) in other {
                let paths = maybe_libraries
                    .iter()
                    .map(|maybe_library| maybe_library.build(opts))
                    .collect::<Result<Vec<_>>>()?;
                toolchain_map
                    .entry(toolchain.clone())
                    .or_default()
                    .extend(paths);
            }
        }
    }

    for name in &lib_sel.libs {
        ensure!(!lib_sel.all, "`--lib` cannot be used with `--all`");
        let (toolchain, maybe_library) =
            name_as_lib(name_toolchain_map, name, true)?.unwrap_or_else(|| unreachable!());
        let path = maybe_library.build(opts)?;
        toolchain_map.entry(toolchain).or_default().insert(path);
    }

    for name in &lib_sel.lib_paths {
        let (toolchain, path) = name_as_path(name, true)?.unwrap_or_else(|| unreachable!());
        toolchain_map.entry(toolchain).or_default().insert(path);
    }

    Ok(toolchain_map)
}

pub fn name_as_lib(
    name_toolchain_map: &NameToolchainMap,
    name: &str,
    as_lib_only: bool,
) -> Result<Option<(String, MaybeLibrary)>> {
    if !is_valid_lib_name(name) {
        ensure!(!as_lib_only, "`{name}` is not a valid library name");
        return Ok(None);
    }

    let name_toolchain_map = name_toolchain_map.get_or_try_init()?;

    if let Some(toolchain_map) = name_toolchain_map.get(name) {
        let mut toolchain_maybe_libraries = flatten_toolchain_map(toolchain_map);

        return match toolchain_maybe_libraries.len() {
            0 => Ok(None),
            1 => Ok(Some(toolchain_maybe_libraries.remove(0))),
            _ => Err(anyhow!(
                "Found multiple libraries matching `{}`: {:?}",
                name,
                toolchain_maybe_libraries
                    .iter()
                    .map(|(_, path)| path)
                    .collect::<Vec<_>>()
            )),
        };
    }

    ensure!(!as_lib_only, "Could not find `--lib {name}`");

    Ok(None)
}

fn is_valid_lib_name(name: &str) -> bool {
    Path::new(name).file_name() == Some(OsStr::new(name))
}

fn flatten_toolchain_map<I, T>(toolchain_map: &BTreeMap<String, I>) -> Vec<(String, T)>
where
    for<'a> &'a I: IntoIterator<Item = &'a T>,
    T: Clone,
{
    let mut result = Vec::new();
    for (toolchain, values) in toolchain_map {
        for value in values {
            result.push((toolchain.clone(), value.clone()));
        }
    }
    result
}

fn name_as_path(name: &str, as_path_only: bool) -> Result<Option<(String, PathBuf)>> {
    if let Ok(path) = PathBuf::from(name).canonicalize() {
        if let Some((_, toolchain)) = parse_path_filename(&path) {
            return Ok(Some((toolchain, path)));
        }

        ensure!(
            !as_path_only,
            "`--lib-path {name}` was used, but the filename does not have the required form: {}",
            *REQUIRED_FORM
        );

        // smoelius: If `name` contains a path separator, then it was clearly meant to be a
        // path.
        ensure!(
            !name.contains(MAIN_SEPARATOR),
            "`{name}` is a valid path, but the filename does not have the required form: {}",
            *REQUIRED_FORM
        );

        ensure!(
            !as_path_only,
            "`--lib-path {name}` was used, but it is invalid"
        );
    }

    ensure!(!as_path_only, "Could not find `--path {name}`");

    Ok(None)
}

fn check_or_fix(
    opts: &opts::Dylint,
    check_opts: &opts::Check,
    resolved: &ToolchainMap,
) -> Result<()> {
    let clippy_disable_docs_links = clippy_disable_docs_links()?;

    let mut failures = Vec::new();

    for (toolchain, paths) in resolved {
        let target_dir = target_dir(opts, toolchain)?;
        let target_dir_str = target_dir.to_string_lossy();
        let driver = driver_builder::get(opts, toolchain)?;
        let dylint_libs = serde_json::to_string(&paths)?;
        #[cfg(not(__library_packages))]
        let dylint_metadata = None;
        #[cfg(__library_packages)]
        let dylint_metadata = library_packages::dylint_metadata(opts)?;
        let dylint_metadata_str = dylint_metadata
            .map(|object: &Object| serde_json::Value::from(object.clone()))
            .unwrap_or_default()
            .to_string();
        let description = format!("with toolchain `{toolchain}`");
        let mut command = if check_opts.fix {
            dylint_internal::cargo::fix(&description)
        } else {
            dylint_internal::cargo::check(&description)
        }
        .build();
        let mut args = vec!["--target-dir", &target_dir_str];
        if let Some(path) = &check_opts.lib_sel.manifest_path {
            args.extend(["--manifest-path", path]);
        }
        for spec in &check_opts.packages {
            args.extend(["-p", spec]);
        }
        if check_opts.workspace {
            args.extend(["--workspace"]);
        }
        args.extend(check_opts.args.iter().map(String::as_str));

        // smoelius: Set CLIPPY_DISABLE_DOCS_LINKS to prevent lints from accidentally linking to the
        // Clippy repository. But set it to the JSON-encoded original value so that the Clippy
        // library can unset the variable.
        // smoelius: This doesn't work if another library is loaded alongside Clippy.
        // smoelius: This was fixed in `clippy_utils`:
        // https://github.com/rust-lang/rust-clippy/commit/1a206fc4abae0b57a3f393481367cf3efca23586
        // But I am going to continue to set CLIPPY_DISABLE_DOCS_LINKS because it doesn't seem to
        // hurt and it provides a small amount of backward compatibility.
        command
            .sanitize_environment()
            .envs([
                (
                    env::CLIPPY_DISABLE_DOCS_LINKS,
                    clippy_disable_docs_links.as_str(),
                ),
                (env::DYLINT_LIBS, &dylint_libs),
                (env::DYLINT_METADATA, &dylint_metadata_str),
                (
                    env::DYLINT_NO_DEPS,
                    if check_opts.no_deps { "1" } else { "0" },
                ),
                (env::RUSTC_WORKSPACE_WRAPPER, &*driver.to_string_lossy()),
                (env::RUSTUP_TOOLCHAIN, toolchain),
            ])
            .args(args);

        // smoelius:: See: https://github.com/rust-lang/rustup/pull/3703 and
        // https://github.com/rust-lang/rustup/issues/3825
        #[cfg(windows)]
        {
            let new_path = dylint_internal::prepend_toolchain_path(toolchain)?;
            command.envs(vec![(crate::env::PATH, new_path)]);
        }

        if let Some(stderr_path) = &opts.pipe_stderr {
            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(stderr_path)
                .with_context(|| format!("Failed to open `{stderr_path}` for stderr usage"))?;
            command.stderr(file);
        }

        if let Some(stdout_path) = &opts.pipe_stdout {
            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(stdout_path)
                .with_context(|| format!("Failed to open `{stdout_path}` for stdout usage"))?;
            command.stdout(file);
        }

        let result = command.success();
        if result.is_err() {
            if !check_opts.keep_going {
                return result
                    .with_context(|| format!("Compilation failed with toolchain `{toolchain}`"));
            }
            failures.push(toolchain);
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "Compilation failed with the following toolchains: {failures:?}"
        ))
    }
}

fn list_lints(opts: &opts::Dylint, resolved: &ToolchainMap) -> Result<()> {
    for (toolchain, paths) in resolved {
        for path in paths {
            let driver = driver_builder::get(opts, toolchain)?;
            let dylint_libs = serde_json::to_string(&[path])?;
            let (name, _) =
                parse_path_filename(path).ok_or_else(|| anyhow!("Could not parse path"))?;

            print!("{name}");
            if resolved.keys().len() >= 2 {
                print!("@{toolchain}");
            }
            if paths.len() >= 2 {
                let location = display_location(path)?;
                print!(" ({location})");
            }
            println!();

            // smoelius: `-W help` is the normal way to list lints, so we can be sure it
            // gets the lints loaded. However, we don't actually use it to list the lints.
            let mut command = dylint_driver(toolchain, &driver)?;
            command
                .envs([
                    (env::DYLINT_LIBS, dylint_libs.as_str()),
                    (env::DYLINT_LIST, "1"),
                ])
                .args(["rustc", "-W", "help"])
                .success()?;

            println!();
        }
    }

    Ok(())
}

fn display_location(path: &Path) -> Result<String> {
    let current_dir = current_dir().with_context(|| "Could not get current directory")?;
    let Ok(path_buf) = path.canonicalize() else {
        return Ok("<unbuilt>".to_owned());
    };
    let parent = path_buf
        .parent()
        .ok_or_else(|| anyhow!("Could not get parent directory"))?;
    Ok(parent
        .strip_prefix(&current_dir)
        .unwrap_or(parent)
        .to_string_lossy()
        .to_string())
}

fn clippy_disable_docs_links() -> Result<String> {
    let val = env::var(env::CLIPPY_DISABLE_DOCS_LINKS).ok();
    serde_json::to_string(&val).map_err(Into::into)
}

fn target_dir(opts: &opts::Dylint, toolchain: &str) -> Result<PathBuf> {
    let mut command = MetadataCommand::new();
    if let Some(path) = &opts.library_selection().manifest_path {
        command.manifest_path(path);
    }
    let metadata = command.no_deps().exec()?;
    Ok(metadata
        .target_directory
        .join("dylint/target")
        .join(toolchain)
        .into())
}

#[allow(clippy::unwrap_used)]
#[cfg(all(test, __library_packages))]
mod test {
    use super::*;
    use std::sync::Mutex;

    // smoelius: With the upgrade to nightly-2023-03-10, I started running into this:
    // https://github.com/rust-lang/rustup/issues/988
    // The easiest solution is to just run the tests sequentially.
    static MUTEX: Mutex<()> = Mutex::new(());

    static OPTS: LazyLock<opts::Dylint> = LazyLock::new(|| opts::Dylint {
        operation: opts::Operation::Check(opts::Check {
            lib_sel: opts::LibrarySelection {
                manifest_path: Some(String::from("../fixtures/name_toolchain_map/Cargo.toml")),
                ..Default::default()
            },
            ..Default::default()
        }),
        ..Default::default()
    });

    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    #[test]
    fn multiple_libraries_multiple_toolchains() {
        let _lock = MUTEX.lock().unwrap();

        let name_toolchain_map = name_toolchain_map();

        let inited = name_toolchain_map.get_or_try_init().unwrap();

        let question_mark_in_expression = inited.get("question_mark_in_expression").unwrap();
        let straggler = inited.get("straggler").unwrap();

        assert_ne!(
            question_mark_in_expression.keys().collect::<Vec<_>>(),
            straggler.keys().collect::<Vec<_>>()
        );

        let opts = opts::Dylint {
            operation: opts::Operation::Check(opts::Check {
                lib_sel: opts::LibrarySelection {
                    libs: vec![
                        "question_mark_in_expression".to_owned(),
                        "straggler".to_owned(),
                    ],
                    ..Default::default()
                },
                ..Default::default()
            }),
            ..Default::default()
        };

        run_with_name_toolchain_map(&opts, &name_toolchain_map).unwrap();
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
    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    #[test]
    fn multiple_libraries_one_toolchain() {
        let _lock = MUTEX.lock().unwrap();

        let name_toolchain_map = name_toolchain_map();

        let inited = name_toolchain_map.get_or_try_init().unwrap();

        let clippy = inited.get("clippy").unwrap();
        let question_mark_in_expression = inited.get("question_mark_in_expression").unwrap();

        assert_eq!(
            clippy.keys().collect::<Vec<_>>(),
            question_mark_in_expression.keys().collect::<Vec<_>>()
        );

        let opts = opts::Dylint {
            operation: opts::Operation::Check(opts::Check {
                lib_sel: opts::LibrarySelection {
                    libs: vec![
                        "clippy".to_owned(),
                        "question_mark_in_expression".to_owned(),
                    ],
                    ..Default::default()
                },
                ..Default::default()
            }),
            ..Default::default()
        };

        run_with_name_toolchain_map(&opts, &name_toolchain_map).unwrap();
    }

    fn name_toolchain_map() -> NameToolchainMap<'static> {
        NameToolchainMap::new(&OPTS)
    }
}
