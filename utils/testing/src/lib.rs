use anyhow::{anyhow, ensure, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package, Target};
use compiletest_rs::{self as compiletest, common::Mode as TestMode};
use dylint_internal::{
    env::{self, var},
    Command,
};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    env::set_var,
    fs::{copy, read_dir, remove_file},
    io::BufRead,
    path::Path,
    path::PathBuf,
    sync::Mutex,
};

pub fn ui_test(name: &str, src_base: &Path) {
    let driver = initialize(name).unwrap();

    run_tests(&driver, None, src_base);
}

pub fn ui_test_example(name: &str, example: &str) {
    let driver = initialize(name).unwrap();

    let metadata = metadata().unwrap();
    let package = package(&metadata).unwrap();
    let target = example_target(&package, example).unwrap();

    run_example_test(&driver, &metadata, &package, &target).unwrap();
}

pub fn ui_test_examples(name: &str) {
    let driver = initialize(name).unwrap();

    let metadata = metadata().unwrap();
    let package = package(&metadata).unwrap();
    let targets = example_targets(&package).unwrap();

    for target in targets {
        run_example_test(&driver, &metadata, &package, &target).unwrap();
    }
}

fn initialize(name: &str) -> Result<PathBuf> {
    let _ = env_logger::builder().try_init();

    // smoelius: Try to order failures by how informative they are: failure to build the library,
    // failure to find the library, failure to build/find the driver.

    dylint_internal::build().success()?;

    // smoelius: `DYLINT_LIBRARY_PATH` must be set before `dylint_libs` is called.
    let manifest_dir = var(env::CARGO_MANIFEST_DIR)?;
    set_var(
        env::DYLINT_LIBRARY_PATH,
        Path::new(&manifest_dir).join("target").join("debug"),
    );

    let dylint_libs = dylint_libs(name)?;
    let driver = dylint::driver_builder::get(&dylint::Dylint::default(), env!("RUSTUP_TOOLCHAIN"))?;

    set_var(env::CLIPPY_DISABLE_DOCS_LINKS, "true");
    set_var(env::DYLINT_LIBS, dylint_libs);

    Ok(driver)
}

pub fn dylint_libs(name: &str) -> Result<String> {
    let name_toolchain_map = dylint::name_toolchain_map(&dylint::Dylint::default())?;
    let entry = dylint::name_as_lib(&name_toolchain_map, name, false)?;
    let (_, path) = entry.ok_or_else(|| anyhow!("Could not find library `{}`", name))?;
    let paths = vec![path];
    serde_json::to_string(&paths).map_err(Into::into)
}

fn metadata() -> Result<Metadata> {
    let manifest_dir = var(env::CARGO_MANIFEST_DIR)?;
    let manifest_path = Path::new(&manifest_dir).join("Cargo.toml");
    MetadataCommand::new()
        .manifest_path(manifest_path)
        .no_deps()
        .exec()
        .map_err(Into::into)
}

fn package(metadata: &Metadata) -> Result<Package> {
    ensure!(metadata.packages.len() <= 1, "Found multiple packages");
    metadata
        .packages
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("Found no packages"))
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
) -> Result<()> {
    let linking_flags = linking_flags(metadata, package, target)?;
    let file_name = target
        .src_path
        .file_name()
        .ok_or_else(|| anyhow!("Could not get file name"))?;

    let tempdir = tempfile::tempdir()?;
    let src_base = tempdir.path();
    let to = src_base.join(file_name);

    copy(&target.src_path, &to)?;
    copy_with_extension(&target.src_path, &to, "stderr").unwrap_or_default();
    copy_with_extension(&target.src_path, &to, "stdout").unwrap_or_default();

    run_tests(driver, Some(&linking_flags.join(" ")), src_base);

    Ok(())
}

fn linking_flags(metadata: &Metadata, package: &Package, target: &Target) -> Result<Vec<String>> {
    let rustc_flags = rustc_flags(metadata, package, target)?;

    let mut linking_flags = Vec::new();

    let mut iter = rustc_flags.iter();
    while let Some(flag) = iter.next() {
        if flag.starts_with("--edition=") {
            linking_flags.push(flag);
        } else if flag == "--extern" || flag == "-L" {
            let arg = next(flag, &mut iter)?;
            linking_flags.extend_from_slice(&[flag, arg]);
        }
    }

    Ok(linking_flags.into_iter().cloned().collect())
}

// smoelius: We need to recover the `rustc` flags used to build a target. I can see three options:
//
// * Use `cargo build --build-plan`
//   - Pros: Easily parsable JSON output
//   - Cons: Unstable and likely to be removed: https://github.com/rust-lang/cargo/issues/7614
// * Parse the output of `cargo build --verbose`
//   - Pros: ?
//   - Cons: Not as easily parsable, requires synchronization (see below)
// * Set `RUSTC_WORKSPACE_WRAPPER` to something that logs `rustc` invocations
//   - Pros: Ground truth
//   - Cons: Requires a separate executable/script, portability could be an issue
//
// I am going with the second option for now, because it seems to be the least of all evils. This
// decision may need to be revisited.

lazy_static! {
    static ref LOCK: Mutex<()> = Mutex::new(());
    static ref RE: Regex = Regex::new(r"^\s*Running\s*`(.*)`$").unwrap();
}

fn rustc_flags(metadata: &Metadata, package: &Package, target: &Target) -> Result<Vec<String>> {
    // smoelius: Force rebuilding of the example by removing it. This is kind of messy. The example
    // is a shared resource that may be needed by multiple tests. For now, I lock a mutex while the
    // example is removed and put back.
    let output = {
        let _guard = LOCK
            .lock()
            .map_err(|err| anyhow!("Could not take lock: {}", err));

        remove_example(metadata, package, target)?;

        Command::new("cargo")
            .args(&[
                "build",
                "--manifest-path",
                &package.manifest_path.to_string(),
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
            let line = line?;
            Ok((*RE).captures(&line).and_then(|captures| {
                let args = captures[1]
                    .split(' ')
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                if args.first().map(AsRef::as_ref) == Some("rustc")
                    && args
                        .as_slice()
                        .windows(2)
                        .any(|window| window == ["--crate-name", &target.name])
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
    for entry in read_dir(metadata.target_directory.join("debug").join("examples"))? {
        let entry = entry?;
        let path = entry.path();

        if let Some(file_name) = path.file_name() {
            let s = file_name.to_string_lossy();
            if s == target.name.clone() + std::env::consts::EXE_SUFFIX
                || s.starts_with(&(target.name.clone() + "-"))
            {
                remove_file(path)?;
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

fn run_tests(driver: &Path, rustc_flags: Option<&str>, src_base: &Path) {
    let config = compiletest::Config {
        mode: TestMode::Ui,
        rustc_path: driver.to_path_buf(),
        src_base: src_base.to_path_buf(),
        target_rustcflags: Some(
            rustc_flags.unwrap_or_default().to_owned() + " --emit=metadata -Dwarnings -Zui-testing",
        ),
        ..compiletest::Config::default()
    };
    compiletest::run_tests(&config);
}
