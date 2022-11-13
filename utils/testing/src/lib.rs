use anyhow::{anyhow, ensure, Context, Result};
use cargo_metadata::{Metadata, Package, Target};
use compiletest_rs as compiletest;
use dylint_internal::{env, library_filename, rustup::is_rustc};
use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use regex::Regex;
use std::{
    env::{consts, remove_var, set_var, var_os},
    ffi::{OsStr, OsString},
    fs::{copy, read_dir, remove_file},
    io::BufRead,
    path::Path,
    path::PathBuf,
    sync::Mutex,
};

pub mod ui;
use ui::Config;

static DRIVER: OnceCell<PathBuf> = OnceCell::new();
static LINKING_FLAGS: OnceCell<Vec<String>> = OnceCell::new();

pub fn ui_test(name: &str, src_base: &Path) {
    ui::Test::src_base(name, src_base).run();
}

pub fn ui_test_example(name: &str, example: &str) {
    ui::Test::example(name, example).run();
}

pub fn ui_test_examples(name: &str) {
    ui::Test::examples(name).run();
}

fn initialize(name: &str) -> Result<&Path> {
    DRIVER
        .get_or_try_init(|| {
            let _ = env_logger::builder().try_init();

            // smoelius: Try to order failures by how informative they are: failure to build the library,
            // failure to find the library, failure to build/find the driver.

            dylint_internal::cargo::build(&format!("library `{name}`"), false).success()?;

            // smoelius: `DYLINT_LIBRARY_PATH` must be set before `dylint_libs` is called.
            // smoelius: This was true when `dylint_libs` called `name_toolchain_map`, but that is no longer
            // the case. I am leaving the comment here for now in case removal of the `name_toolchain_map`
            // call causes a regression.
            let metadata = dylint_internal::cargo::current_metadata().unwrap();
            let dylint_library_path = metadata.target_directory.join("debug");
            set_var(env::DYLINT_LIBRARY_PATH, dylint_library_path);

            let dylint_libs = dylint_libs(name)?;
            let driver =
                dylint::driver_builder::get(&dylint::Dylint::default(), env!("RUSTUP_TOOLCHAIN"))?;

            set_var(env::CLIPPY_DISABLE_DOCS_LINKS, "true");
            set_var(env::DYLINT_LIBS, dylint_libs);

            Ok(driver)
        })
        .map(PathBuf::as_path)
}

pub fn dylint_libs(name: &str) -> Result<String> {
    let metadata = dylint_internal::cargo::current_metadata().unwrap();
    let rustup_toolchain = env::var(env::RUSTUP_TOOLCHAIN)?;
    let filename = library_filename(name, &rustup_toolchain);
    let path = metadata.target_directory.join("debug").join(filename);
    let paths = vec![path];
    serde_json::to_string(&paths).map_err(Into::into)
}

fn example_target(package: &Package, example: &str) -> Result<Target> {
    package
        .targets
        .iter()
        .find(|target| target.kind == ["example"] && target.name == example)
        .cloned()
        .ok_or_else(|| anyhow!("Could not find example `{}`", example))
}

#[allow(clippy::unnecessary_wraps)]
fn example_targets(package: &Package) -> Result<Vec<Target>> {
    Ok(package
        .targets
        .iter()
        .filter(|target| target.kind == ["example"])
        .cloned()
        .collect())
}

fn run_example_test(
    driver: &Path,
    metadata: &Metadata,
    package: &Package,
    target: &Target,
    config: &Config,
) -> Result<()> {
    let linking_flags = linking_flags(metadata, package, target)?;
    let file_name = target
        .src_path
        .file_name()
        .ok_or_else(|| anyhow!("Could not get file name"))?;

    let tempdir = tempfile::tempdir().with_context(|| "`tempdir` failed")?;
    let src_base = tempdir.path();
    let to = src_base.join(file_name);

    copy(&target.src_path, &to).with_context(|| {
        format!(
            "Could not copy `{}` to `{}`",
            target.src_path,
            to.to_string_lossy()
        )
    })?;
    ["fixed", "stderr", "stdout"]
        .map(|extension| copy_with_extension(&target.src_path, &to, extension).unwrap_or_default());

    let mut config = config.clone();
    config.rustc_flags.extend(linking_flags.iter().cloned());

    run_tests(driver, src_base, &config);

    Ok(())
}

fn linking_flags(
    metadata: &Metadata,
    package: &Package,
    target: &Target,
) -> Result<&'static [String]> {
    LINKING_FLAGS
        .get_or_try_init(|| {
            let rustc_flags = rustc_flags(metadata, package, target)?;

            let mut linking_flags = Vec::new();

            let mut iter = rustc_flags.into_iter();
            while let Some(flag) = iter.next() {
                if flag.starts_with("--edition=") {
                    linking_flags.push(flag);
                } else if flag == "--extern" || flag == "-L" {
                    let arg = next(&flag, &mut iter)?;
                    linking_flags.extend(vec![flag, arg.trim_matches('\'').to_owned()]);
                }
            }

            Ok(linking_flags)
        })
        .map(Vec::as_slice)
}

// smoelius: We need to recover the `rustc` flags used to build a target. I can see four options:
//
// * Use `cargo build --build-plan`
//   - Pros: Easily parsable JSON output
//   - Cons: Unstable and likely to be removed: https://github.com/rust-lang/cargo/issues/7614
// * Parse the output of `cargo build --verbose`
//   - Pros: ?
//   - Cons: Not as easily parsable, requires synchronization (see below)
// * Use a custom executor like Siderophile does: https://github.com/trailofbits/siderophile/blob/26c067306f6c2f66d9530dacef6b17dbf59cdf8c/src/trawl_source/mod.rs#L399
//   - Pros: Ground truth
//   - Cons: Seems a bit of a heavy lift
//     Note: I think Siderophile's approach was inspired by `cargo-geiger`.
// * Set `RUSTC_WORKSPACE_WRAPPER` to something that logs `rustc` invocations
//   - Pros: Ground truth
//   - Cons: Requires a separate executable/script, portability could be an issue
//
// I am going with the second option for now, because it seems to be the least of all evils. This
// decision may need to be revisited.

lazy_static! {
    static ref RE: Regex = Regex::new(r"^\s*Running\s*`(.*)`$").unwrap();
}

fn rustc_flags(metadata: &Metadata, package: &Package, target: &Target) -> Result<Vec<String>> {
    // smoelius: The following comments are old and retained for posterity. The linking flags are
    // now initialized using a `OnceCell`, which makes the mutex unnecessary.
    //   smoelius: Force rebuilding of the example by removing it. This is kind of messy. The
    //   example is a shared resource that may be needed by multiple tests. For now, I lock a mutex
    //   while the example is removed and put back.
    //   smoelius: Should we use a temporary target directory here?
    let output = {
        remove_example(metadata, package, target)?;

        // smoelius: Because of lazy initialization, `cargo build` is run only once. Seeing
        // "Building example `target`" for one example but not for others is confusing. So instead
        // say "Building `package` examples".
        dylint_internal::cargo::build(&format!("`{}` examples", package.name), false)
            .envs(vec![(env::CARGO_TERM_COLOR, "never")])
            .args([
                "--manifest-path",
                package.manifest_path.as_ref(),
                "--example",
                &target.name,
                "--verbose",
            ])
            .output()?
    };

    let matches = output
        .stderr
        .lines()
        .map(|line| {
            let line =
                line.with_context(|| format!("Could not read from `{}`", package.manifest_path))?;
            Ok((*RE).captures(&line).and_then(|captures| {
                let args = captures[1]
                    .split(' ')
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                if args.first().map_or(false, is_rustc)
                    && args
                        .as_slice()
                        .windows(2)
                        .any(|window| window == ["--crate-name", &snake_case(&target.name)])
                {
                    Some(args)
                } else {
                    None
                }
            }))
        })
        .collect::<Result<Vec<Option<Vec<_>>>>>()?;

    let mut matches = matches.into_iter().flatten().collect::<Vec<Vec<_>>>();
    ensure!(
        matches.len() <= 1,
        "Found multiple `rustc` invocations for `{}`",
        target.name
    );
    matches
        .pop()
        .ok_or_else(|| anyhow!("Found no `rustc` invocations for `{}`", target.name))
}

fn remove_example(metadata: &Metadata, _package: &Package, target: &Target) -> Result<()> {
    let examples = metadata.target_directory.join("debug").join("examples");
    for entry in
        read_dir(&examples).with_context(|| format!("`read_dir` failed for `{examples}`"))?
    {
        let entry = entry.with_context(|| format!("`read_dir` failed for `{examples}`"))?;
        let path = entry.path();

        if let Some(file_name) = path.file_name() {
            let s = file_name.to_string_lossy();
            let target_name = snake_case(&target.name);
            if s == target_name.clone() + consts::EXE_SUFFIX
                || s.starts_with(&(target_name.clone() + "-"))
            {
                remove_file(&path).with_context(|| {
                    format!("`remove_file` failed for `{}`", path.to_string_lossy())
                })?;
            }
        }
    }

    Ok(())
}

fn next<I, T>(flag: &str, iter: &mut I) -> Result<T>
where
    I: Iterator<Item = T>,
{
    iter.next()
        .ok_or_else(|| anyhow!("Missing argument for `{}`", flag))
}

fn copy_with_extension<P: AsRef<Path>, Q: AsRef<Path>>(
    from: P,
    to: Q,
    extension: &str,
) -> Result<u64> {
    let from = from.as_ref().with_extension(extension);
    let to = to.as_ref().with_extension(extension);
    copy(from, to).map_err(Into::into)
}

lazy_static! {
    static ref MUTEX: Mutex<()> = Mutex::new(());
}

fn run_tests(driver: &Path, src_base: &Path, config: &Config) {
    let _lock = MUTEX.lock().unwrap();

    // smoelius: There doesn't seem to be a way to set environment variables using `compiletest`'s
    // [`Config`](https://docs.rs/compiletest_rs/0.7.1/compiletest_rs/common/struct.Config.html)
    // struct. For comparison, where Clippy uses `compiletest`, it sets environment variables
    // directly (see: https://github.com/rust-lang/rust-clippy/blob/master/tests/compile-test.rs).
    //   Of course, even if `compiletest` had such support, it would need to be incorporated into
    // `dylint_testing`.

    let _var = config
        .dylint_toml
        .as_ref()
        .map(|value| VarGuard::set(env::DYLINT_TOML, value));

    let config = compiletest::Config {
        mode: compiletest::common::Mode::Ui,
        rustc_path: driver.to_path_buf(),
        src_base: src_base.to_path_buf(),
        target_rustcflags: Some(
            config.rustc_flags.clone().join(" ") + " --emit=metadata -Dwarnings -Zui-testing",
        ),
        ..compiletest::Config::default()
    };

    compiletest::run_tests(&config);
}

// smoelius: `VarGuard` was copied from:
// https://github.com/rust-lang/rust-clippy/blob/9cc8da222b3893bc13bc13c8827e93f8ea246854/tests/compile-test.rs

/// Restores an env var on drop
#[must_use]
struct VarGuard {
    key: &'static str,
    value: Option<OsString>,
}

impl VarGuard {
    fn set(key: &'static str, val: impl AsRef<OsStr>) -> Self {
        let value = var_os(key);
        set_var(key, val);
        Self { key, value }
    }
}

impl Drop for VarGuard {
    fn drop(&mut self) {
        match self.value.as_deref() {
            None => remove_var(self.key),
            Some(value) => set_var(self.key, value),
        }
    }
}

fn snake_case(name: &str) -> String {
    name.replace('-', "_")
}
