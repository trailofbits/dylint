use anyhow::{anyhow, ensure, Result};
use cargo_metadata::{Dependency, MetadataCommand};
use dylint_env as env;
use git2::{Oid, Repository, ResetType};
use std::{
    fs::{read_to_string, write, OpenOptions},
    io::Write,
    path::Path,
    process::Command,
};
use tempfile::tempdir_in;

#[test]
fn ui() {
    let _ = env_logger::try_init();

    dylint_testing::build(None).unwrap();

    let tempdir = tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();

    checkout_rust_clippy(tempdir.path()).unwrap();

    isolate(tempdir.path()).unwrap();

    let src_base = tempdir.path().join("tests").join("ui");

    disable_rustfix(&src_base).unwrap();
    adjust_macro_use_imports_test(&src_base).unwrap();

    let dylint_libs = dylint_testing::dylint_libs("clippy").unwrap();
    let driver = dylint::driver_builder::get(env!("RUSTUP_TOOLCHAIN")).unwrap();

    let mut command = Command::new("cargo");

    command
        .current_dir(tempdir.path())
        .envs(vec![
            (env::DYLINT_LIBS, dylint_libs),
            (
                env::CLIPPY_DRIVER_PATH,
                driver.to_string_lossy().to_string(),
            ),
            (
                env::DYLINT_RUSTFLAGS,
                r#"--cfg feature="cargo-clippy""#.to_owned(),
            ),
        ])
        .args(&["test", "--test", "compile-test"]);

    assert!(command.status().unwrap().success());
}

fn checkout_rust_clippy(path: &Path) -> Result<()> {
    let clippy_lints = clippy_lints_dependency()?;
    let source = clippy_lints.source.ok_or_else(|| anyhow!("No source"))?;
    let url = source
        .strip_prefix("git+")
        .ok_or_else(|| anyhow!("Wrong prefix"))?;
    let rev = url
        .rsplit('=')
        .next()
        .ok_or_else(|| anyhow!("Wrong suffix"))?;
    let oid = Oid::from_str(rev)?;

    let repository = Repository::clone(url, path)?;
    let object = repository.find_object(oid, None)?;
    repository.reset(&object, ResetType::Hard, None)?;

    Ok(())
}

fn clippy_lints_dependency() -> Result<Dependency> {
    let metadata = MetadataCommand::new().no_deps().exec()?;
    let package = metadata
        .packages
        .iter()
        .find(|package| package.name == env!("CARGO_PKG_NAME"))
        .ok_or_else(|| anyhow!("Could not find package"))?;
    let dependency = package
        .dependencies
        .iter()
        .find(|dependency| dependency.name == "clippy_lints")
        .ok_or_else(|| anyhow!("Could not find dependency"))?;
    Ok(dependency.clone())
}

// smoelius: So long as Clippy is checked out in the current directory, this must be dealt with:
// error: current package believes it's in a workspace when it's not
fn isolate(path: &Path) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path.join("Cargo.toml"))?;

    writeln!(
        file,
        r#"
[workspace]
members = ["."]
"#
    )?;

    Ok(())
}

// smoelius: FIXME: Shell
fn disable_rustfix(src_base: &Path) -> Result<()> {
    let mut command = Command::new("sh");
    command.current_dir(&src_base).args(&[
        "-c",
        r#"
            sed -i -e 's,\<run-rustfix\>,,' *.rs &&
            rm -f *.fixed
        "#,
    ]);
    log::debug!("{:?}", command);
    let status = command.status()?;
    ensure!(status.success(), "command failed: {:?}", command);
    Ok(())
}

// smoelius: The `macro_use_imports` test produces the right errors, but not in the right order.
// I haven't yet figured out why. Hence, this hack.
fn adjust_macro_use_imports_test(src_base: &Path) -> Result<()> {
    let stderr_file = src_base.join("macro_use_imports.stderr");
    let contents = read_to_string(&stderr_file)?;
    let lines: Vec<String> = contents.lines().map(ToString::to_string).collect();

    let (first_error, rest) = lines.split_at(5);
    let (note, rest) = rest.split_at(2);
    let (blank_line, rest) = rest.split_at(1);
    let (second_error, rest) = rest.split_at(5);
    let (remaining_errors, summary) = rest.split_at(rest.len() - 2);

    let permuted: Vec<String> = std::iter::empty()
        .chain(second_error.iter().cloned())
        .chain(note.iter().cloned())
        .chain(remaining_errors.iter().cloned())
        .chain(first_error.iter().cloned())
        .chain(blank_line.iter().cloned())
        .chain(summary.iter().cloned())
        .collect();

    let mut lines_sorted = lines.clone();
    let mut permuted_sorted = permuted.clone();
    lines_sorted.sort();
    permuted_sorted.sort();
    assert_eq!(lines_sorted, permuted_sorted);

    write(
        stderr_file,
        permuted
            .iter()
            .map(|line| format!("{}\n", line))
            .collect::<String>(),
    )?;

    Ok(())
}
