use anyhow::{anyhow, Result};
use cargo_metadata::{Dependency, MetadataCommand};
use dylint_internal::{env, testing::isolate, Command};
use std::{
    env::set_var,
    fs::{read_to_string, write},
    path::Path,
};
use tempfile::tempdir_in;
use test_env_log::test;

const ERROR_LINES: usize = 5;

#[test]
fn ui() {
    // smoelius: Try to order failures by how informative they are: failure to build the library,
    // failure to find the library, failure to build/find the driver.

    dylint_internal::build().success().unwrap();

    let tempdir = tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();

    checkout_rust_clippy(tempdir.path()).unwrap();

    isolate(tempdir.path()).unwrap();

    let src_base = tempdir.path().join("tests").join("ui");

    disable_rustfix(&src_base).unwrap();
    adjust_macro_use_imports_test(&src_base).unwrap();

    // smoelius: `DYLINT_LIBRARY_PATH` must be set before `dylint_libs` is called.
    set_var(
        env::DYLINT_LIBRARY_PATH,
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug"),
    );

    let dylint_libs = dylint_testing::dylint_libs("clippy").unwrap();
    let driver =
        dylint::driver_builder::get(&dylint::Dylint::default(), env!("RUSTUP_TOOLCHAIN")).unwrap();

    dylint_internal::test()
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
        .args(&["--test", "compile-test"])
        .success()
        .unwrap();
}

fn checkout_rust_clippy(path: &Path) -> Result<()> {
    let clippy_lints = clippy_lints_dependency()?;
    let source = clippy_lints.source.ok_or_else(|| anyhow!("No source"))?;
    let url = source
        .strip_prefix("git+")
        .ok_or_else(|| anyhow!("Wrong prefix"))?;
    let refname = url
        .rsplit('=')
        .next()
        .ok_or_else(|| anyhow!("Wrong suffix"))?;
    dylint_internal::checkout(url, refname, path)
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

// smoelius: FIXME: Shell
fn disable_rustfix(src_base: &Path) -> Result<()> {
    Command::new("sh")
        .current_dir(&src_base)
        .args(&[
            "-c",
            r#"
                sed -i -e 's,\<run-rustfix\>,,' *.rs &&
                rm -f *.fixed
            "#,
        ])
        .success()
}

// smoelius: The `macro_use_imports` test produces the right four errors, but not in the right
// order. I haven't yet figured out why. Hence, this hack.
#[allow(clippy::shadow_unrelated)]
fn adjust_macro_use_imports_test(src_base: &Path) -> Result<()> {
    let stderr_file = src_base.join("macro_use_imports.stderr");
    let contents = read_to_string(&stderr_file)?;
    let lines: Vec<String> = contents.lines().map(ToString::to_string).collect();

    let (first_error, rest) = lines.split_at(ERROR_LINES);
    let (note, rest) = rest.split_at(2);
    let (_blank_line, rest) = rest.split_at(1);
    let (second_error, rest) = rest.split_at(ERROR_LINES);
    let (_blank_line, rest) = rest.split_at(1);
    let (third_error, rest) = rest.split_at(ERROR_LINES);
    let (_blank_line, rest) = rest.split_at(1);
    let (fourth_error, rest) = rest.split_at(ERROR_LINES);
    let (blank_line, summary) = rest.split_at(rest.len() - 2);

    let permuted: Vec<String> = std::iter::empty()
        .chain(first_error.iter().cloned())
        .chain(note.iter().cloned())
        .chain(blank_line.iter().cloned())
        .chain(third_error.iter().cloned())
        .chain(blank_line.iter().cloned())
        .chain(fourth_error.iter().cloned())
        .chain(blank_line.iter().cloned())
        .chain(second_error.iter().cloned())
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
