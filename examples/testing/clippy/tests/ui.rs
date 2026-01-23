use anyhow::{Result, anyhow};
use cargo_metadata::Dependency;
use dylint_internal::{CommandExt, clone, env};
use std::path::Path;
use tempfile::{tempdir, tempdir_in};

#[cfg_attr(dylint_lib = "supplementary", expect(commented_out_code))]
#[test]
fn ui() {
    // smoelius: Try to order failures by how informative they are: failure to build the library,
    // failure to find the library, failure to build/find the driver.

    dylint_internal::cargo::build("clippy")
        .build()
        .success()
        .unwrap();

    let tempdir = tempdir().unwrap();

    clone_rust_clippy(tempdir.path()).unwrap();

    // smoelius: `adjust_macro_use_imports_test` no longer seems to be necessary.
    // let src_base = tempdir.path().join("tests/ui");
    // adjust_macro_use_imports_test(&src_base).unwrap();

    // smoelius: The `5041_allow_dev_build` test is flaky on Windows. See:
    // https://github.com/rust-lang/rust-clippy/issues/11489
    // Disable the test for now.
    #[cfg(windows)]
    std::fs::remove_dir_all(
        tempdir
            .path()
            .join("tests/ui-cargo/multiple_crate_versions/5041_allow_dev_build"),
    )
    .unwrap();

    let dylint_libs = dylint_testing::dylint_libs("clippy").unwrap();
    let driver =
        dylint::driver_builder::get(&dylint::opts::Dylint::default(), env!("RUSTUP_TOOLCHAIN"))
            .unwrap();

    // smoelius: Clippy's `compile-test` panics if multiple rlibs exist for certain crates (see
    // `third_party_crates` in
    // https://github.com/rust-lang/rust-clippy/blob/master/tests/compile-test.rs). This can happen
    // as a result of using a shared target directory. The workaround I have adopted is to use a
    // temporary target directory.
    let target_dir = tempdir_in(".").unwrap();

    // smoelius: A non-canonical temporary current directory seems to cause problems for `ui_test`.
    let tempdir_path = tempdir.path().canonicalize().unwrap();

    let mut command = dylint_internal::cargo::test("clippy").build();
    command
        .current_dir(tempdir_path)
        .envs([
            (env::CARGO_TARGET_DIR, &*target_dir.path().to_string_lossy()),
            (env::DYLINT_LIBS, &dylint_libs),
            (env::CLIPPY_DRIVER_PATH, &*driver.to_string_lossy()),
            (env::DYLINT_RUSTFLAGS, r#"--cfg feature="cargo-clippy""#),
        ])
        .args(["--test", "compile-test"]);

    // smoelius: Error messages like the following have occurred in Windows GitHub workflows:
    //   LINK : fatal error LNK1318: Unexpected PDB error; OK (0) 'D:\a\dylint\dylint\examples\
    //     testing\clippy\...\debug\test\ui\useless_attribute.stage-id.aux\proc_macro_derive.pdb'
    // According to Microsoft Learn, "This error message is produced for uncommon issues in PDB
    // files":
    // https://learn.microsoft.com/en-us/cpp/error-messages/tool-errors/linker-tools-error-lnk1318?view=msvc-170
    // While I don't know the underlying cause, my approach to this problem is to not link PDB
    // files. Taken from here:
    // https://github.com/rust-lang/rust/issues/67012#issuecomment-561801877
    #[cfg(windows)]
    command.envs([(env::RUSTFLAGS, "-C link-arg=/DEBUG:NONE")]);

    command.success().unwrap();
}

fn clone_rust_clippy(path: &Path) -> Result<()> {
    let clippy_lints = clippy_lints_dependency()?;
    let source = clippy_lints.source.ok_or_else(|| anyhow!("No source"))?;
    let url = source
        .repr
        .strip_prefix("git+")
        .ok_or_else(|| anyhow!("Wrong prefix"))?;
    let (url, refname) = url
        .rsplit_once('=')
        .and_then(|(url, refname)| url.rsplit_once('?').map(|(url, _)| (url, refname)))
        .ok_or_else(|| anyhow!("Wrong suffix"))?;
    clone(url, refname, path, false)?;
    Ok(())
}

fn clippy_lints_dependency() -> Result<Dependency> {
    let metadata = dylint_internal::cargo::current_metadata()?;
    let package = metadata
        .packages
        .into_iter()
        .find(|package| package.name.as_str() == env!("CARGO_PKG_NAME"))
        .ok_or_else(|| anyhow!("Could not find package"))?;
    let dependency = package
        .dependencies
        .into_iter()
        .find(|dependency| dependency.name == "clippy_lints")
        .ok_or_else(|| anyhow!("Could not find dependency"))?;
    Ok(dependency)
}

#[cfg(any())]
mod unused {
    use anyhow::{Context, Result};
    use std::{
        fs::{read_to_string, write},
        path::Path,
    };

    const ERROR_LINES: usize = 5;

    // smoelius: The `macro_use_imports` test produces the right four errors, but not in the right
    // order. I haven't yet figured out why. Hence, this hack.
    #[expect(clippy::shadow_unrelated)]
    fn adjust_macro_use_imports_test(src_base: &Path) -> Result<()> {
        let stderr_file = src_base.join("macro_use_imports.stderr");
        let contents = read_to_string(&stderr_file).with_context(|| {
            format!(
                "`read_to_string` failed for `{}`",
                stderr_file.to_string_lossy()
            )
        })?;
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

        let mut permuted = Vec::new();
        permuted.extend(first_error.iter().cloned());
        permuted.extend(note.iter().cloned());
        permuted.extend(blank_line.iter().cloned());
        permuted.extend(second_error.iter().cloned());
        permuted.extend(blank_line.iter().cloned());
        permuted.extend(third_error.iter().cloned());
        permuted.extend(blank_line.iter().cloned());
        permuted.extend(fourth_error.iter().cloned());
        permuted.extend(blank_line.iter().cloned());
        permuted.extend(summary.iter().cloned());

        let mut lines_sorted = lines.clone();
        let mut permuted_sorted = permuted.clone();
        lines_sorted.sort();
        permuted_sorted.sort();
        assert_eq!(lines_sorted, permuted_sorted);

        write(
            &stderr_file,
            permuted
                .iter()
                .map(|line| format!("{line}\n"))
                .collect::<String>(),
        )
        .with_context(|| format!("Could not write to `{}`", stderr_file.to_string_lossy()))?;

        Ok(())
    }
}
