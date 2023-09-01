#![allow(deprecated)]
#![cfg_attr(dylint_lib = "general", allow(crate_wide_allow))]
#![deny(clippy::expect_used)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]

use anyhow::{anyhow, bail, ensure, Context, Result};
use cargo_metadata::MetadataCommand;
use dylint_internal::{
    driver as dylint_driver, env, parse_path_filename, rustup::SanitizeEnvironment,
};
use once_cell::sync::Lazy;
use std::{
    collections::BTreeMap,
    env::{consts, current_dir},
    ffi::OsStr,
    fmt::Debug,
    path::{Path, PathBuf, MAIN_SEPARATOR},
};

type Object = serde_json::Map<String, serde_json::Value>;

#[cfg(feature = "metadata")]
pub(crate) use cargo::{core, sources, util};

pub mod driver_builder;

mod error;
use error::warn;
#[doc(hidden)]
pub use error::warn as __warn;
pub use error::{ColorizedError, ColorizedResult};

mod name_toolchain_map;
pub use name_toolchain_map::{Lazy as NameToolchainMap, ToolchainMap};
use name_toolchain_map::{LazyToolchainMap, MaybeLibrary};

#[cfg(feature = "metadata")]
pub(crate) mod metadata;

#[cfg(feature = "metadata")]
mod toml;

#[cfg(feature = "package_options")]
mod package_options;

static REQUIRED_FORM: Lazy<String> = Lazy::new(|| {
    format!(
        r#""{}" LIBRARY_NAME "@" TOOLCHAIN "{}""#,
        consts::DLL_PREFIX,
        consts::DLL_SUFFIX
    )
});

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default)]
pub struct Dylint {
    pub all: bool,

    #[deprecated]
    pub allow_downgrade: bool,

    #[deprecated]
    pub bisect: bool,

    pub fix: bool,

    #[deprecated]
    pub force: bool,

    #[deprecated]
    pub isolate: bool,

    pub keep_going: bool,

    pub libs: Vec<String>,

    #[deprecated]
    pub list: bool,

    pub manifest_path: Option<String>,

    #[deprecated]
    pub new_path: Option<String>,

    pub no_build: bool,

    pub no_deps: bool,

    pub no_metadata: bool,

    pub packages: Vec<String>,

    pub paths: Vec<String>,

    pub quiet: bool,

    #[deprecated]
    pub rust_version: Option<String>,

    #[deprecated]
    pub upgrade_path: Option<String>,

    pub workspace: bool,

    #[deprecated]
    pub names: Vec<String>,

    pub args: Vec<String>,
}

pub fn run(opts: &Dylint) -> Result<()> {
    let opts = {
        if opts.force {
            warn(
                opts,
                "`--force` is deprecated and its meaning may change in the future. Use \
                 `--allow-downgrade`.",
            );
        }
        Dylint {
            allow_downgrade: opts.allow_downgrade || opts.force,
            ..opts.clone()
        }
    };

    if opts.allow_downgrade && opts.upgrade_path.is_none() {
        bail!("`--allow-downgrade` can be used only with `--upgrade`");
    }

    if opts.bisect {
        #[cfg(not(unix))]
        bail!("`--bisect` is supported only on Unix platforms");

        #[cfg(unix)]
        warn(&opts, "`--bisect` is experimental");
    }

    if opts.bisect && opts.upgrade_path.is_none() {
        bail!("`--bisect` can be used only with `--upgrade`");
    }

    if opts.isolate && opts.new_path.is_none() {
        bail!("`--isolate` can be used only with `--new`");
    }

    if opts.rust_version.is_some() && opts.upgrade_path.is_none() {
        bail!("`--rust-version` can be used only with `--upgrade`");
    }

    #[cfg(feature = "package_options")]
    if let Some(path) = &opts.new_path {
        return package_options::new_package(&opts, Path::new(path));
    }

    #[cfg(feature = "package_options")]
    if let Some(path) = &opts.upgrade_path {
        return package_options::upgrade_package(&opts, Path::new(path));
    }

    let name_toolchain_map = NameToolchainMap::new(&opts);

    run_with_name_toolchain_map(&opts, &name_toolchain_map)
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
fn resolve(opts: &Dylint, name_toolchain_map: &NameToolchainMap) -> Result<ToolchainMap> {
    let mut toolchain_map = ToolchainMap::new();

    if opts.all {
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

    for name in &opts.libs {
        ensure!(!opts.all, "`--lib` cannot be used with `--all`");
        let (toolchain, maybe_library) =
            name_as_lib(name_toolchain_map, name, true)?.unwrap_or_else(|| unreachable!());
        let path = maybe_library.build(opts)?;
        toolchain_map.entry(toolchain).or_default().insert(path);
    }

    for name in &opts.paths {
        let (toolchain, path) = name_as_path(name, true)?.unwrap_or_else(|| unreachable!());
        toolchain_map.entry(toolchain).or_default().insert(path);
    }

    let mut not_found = Vec::new();

    for name in &opts.names {
        if let Some((toolchain, maybe_library)) = name_as_lib(name_toolchain_map, name, false)? {
            ensure!(
                !opts.all,
                "`{}` is a library name and cannot be used with `--all`; if a path was meant, use \
                 `--path {}`",
                name,
                name
            );
            let path = maybe_library.build(opts)?;
            toolchain_map.entry(toolchain).or_default().insert(path);
        } else if let Some((toolchain, path)) = name_as_path(name, false)? {
            toolchain_map.entry(toolchain).or_default().insert(path);
        } else {
            not_found.push(name);
        }
    }

    #[allow(clippy::format_collect)]
    if !not_found.is_empty() {
        not_found.sort_unstable();
        bail!(
            "Could not find the following libraries:{}",
            not_found
                .iter()
                .map(|name| format!("\n    {name}"))
                .collect::<String>()
        );
    }

    Ok(toolchain_map)
}

pub fn name_as_lib(
    name_toolchain_map: &NameToolchainMap,
    name: &str,
    as_lib_only: bool,
) -> Result<Option<(String, MaybeLibrary)>> {
    if !is_valid_lib_name(name) {
        ensure!(!as_lib_only, "`{}` is not a valid library name", name);
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

    ensure!(!as_lib_only, "Could not find `--lib {}`", name);

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
    toolchain_map
        .iter()
        .flat_map(|(toolchain, values)| {
            values
                .into_iter()
                .map(|value| (toolchain.clone(), value.clone()))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn name_as_path(name: &str, as_path_only: bool) -> Result<Option<(String, PathBuf)>> {
    if let Ok(path) = PathBuf::from(name).canonicalize() {
        if let Some((_, toolchain)) = parse_path_filename(&path) {
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
            !name.contains(MAIN_SEPARATOR),
            "`{}` is a valid path, but the filename does not have the required form: {}",
            name,
            *REQUIRED_FORM
        );

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
    let path_buf = match path.canonicalize() {
        Ok(path_buf) => path_buf,
        Err(_) => {
            return Ok("<unbuilt>".to_owned());
        }
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

fn check_or_fix(opts: &Dylint, resolved: &ToolchainMap) -> Result<()> {
    let clippy_disable_docs_links = clippy_disable_docs_links()?;

    let mut failures = Vec::new();

    for (toolchain, paths) in resolved {
        let target_dir = target_dir(opts, toolchain)?;
        let target_dir_str = target_dir.to_string_lossy();
        let driver = driver_builder::get(opts, toolchain)?;
        let dylint_libs = serde_json::to_string(&paths)?;
        #[cfg(not(feature = "metadata"))]
        let dylint_metadata = None;
        #[cfg(feature = "metadata")]
        let dylint_metadata = metadata::dylint_metadata(opts)?;
        let dylint_metadata_str = dylint_metadata
            .map(|object: &Object| serde_json::Value::from(object.clone()))
            .unwrap_or_default()
            .to_string();
        let description = format!("with toolchain `{toolchain}`");
        let mut command = if opts.fix {
            dylint_internal::cargo::fix(&description)
        } else {
            dylint_internal::cargo::check(&description)
        };
        let mut args = vec!["--target-dir", &target_dir_str];
        if let Some(path) = &opts.manifest_path {
            args.extend(["--manifest-path", path]);
        }
        for spec in &opts.packages {
            args.extend(["-p", spec]);
        }
        if opts.workspace {
            args.extend(["--workspace"]);
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
            .sanitize_environment()
            .envs([
                (
                    env::CLIPPY_DISABLE_DOCS_LINKS,
                    clippy_disable_docs_links.as_str(),
                ),
                (env::DYLINT_LIBS, &dylint_libs),
                (env::DYLINT_METADATA, &dylint_metadata_str),
                (env::DYLINT_NO_DEPS, if opts.no_deps { "1" } else { "0" }),
                (env::RUSTC_WORKSPACE_WRAPPER, &*driver.to_string_lossy()),
                (env::RUSTUP_TOOLCHAIN, toolchain),
            ])
            .args(args)
            .success();
        if result.is_err() {
            if !opts.keep_going {
                return result
                    .with_context(|| format!("Compilation failed with toolchain `{toolchain}`"));
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
        .join("dylint/target")
        .join(toolchain)
        .into())
}

fn clippy_disable_docs_links() -> Result<String> {
    let val = env::var(env::CLIPPY_DISABLE_DOCS_LINKS).ok();
    serde_json::to_string(&val).map_err(Into::into)
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use dylint_internal::examples;
    use once_cell::sync::Lazy;
    use std::{
        env::{join_paths, set_var},
        sync::Mutex,
    };

    // smoelius: With the upgrade to nightly-2023-03-10, I started running into this:
    // https://github.com/rust-lang/rustup/issues/988
    // The easiest solution is to just not run the tests concurrently.
    static MUTEX: Mutex<()> = Mutex::new(());

    static OPTS: Lazy<Dylint> = Lazy::new(|| Dylint {
        no_metadata: true,
        ..Dylint::default()
    });

    fn name_toolchain_map() -> NameToolchainMap<'static> {
        examples::build().unwrap();
        let metadata = dylint_internal::cargo::current_metadata().unwrap();
        // smoelius: As of version 0.1.14, `cargo-llvm-cov` no longer sets `CARGO_TARGET_DIR`.
        // So `dylint_library_path` no longer requires a `cfg!(coverage)` special case.
        let dylint_library_path = join_paths([
            metadata.target_directory.join("examples/debug"),
            metadata.target_directory.join("straggler/debug"),
        ])
        .unwrap();

        #[rustfmt::skip]
        // smoelius: Following the upgrade nightly-2023-08-24, I started seeing he following error:
        //
        //   error: internal compiler error: encountered incremental compilation error with shallow_lint_levels_on(dylint_internal[...]::cargo::{use#15})
        //     |
        //     = help: This is a known issue with the compiler. Run `cargo clean -p dylint_internal` or `cargo clean` to allow your project to compile
        //     = note: Please follow the instructions below to create a bug report with the provided information
        //     = note: See <https://github.com/rust-lang/rust/issues/84970> for more information
        set_var(env::CARGO_INCREMENTAL, "0");
        set_var(env::DYLINT_LIBRARY_PATH, dylint_library_path);

        NameToolchainMap::new(&OPTS)
    }

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

        let opts = Dylint {
            libs: vec![
                "question_mark_in_expression".to_owned(),
                "straggler".to_owned(),
            ],
            ..Dylint::default()
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

        let opts = Dylint {
            libs: vec![
                "clippy".to_owned(),
                "question_mark_in_expression".to_owned(),
            ],
            ..Dylint::default()
        };

        run_with_name_toolchain_map(&opts, &name_toolchain_map).unwrap();
    }
}
