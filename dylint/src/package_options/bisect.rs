use crate::Dylint;
use anyhow::{anyhow, Context, Result};
use atty::Stream;
use dylint_internal::{rustup::SanitizeEnvironment, Command};
use std::os::unix::fs::PermissionsExt;
use std::{
    fs::{remove_file, rename},
    io::Write,
    path::Path,
    process::Stdio,
};
use tempfile::NamedTempFile;

const SCRIPT: &str = r#"#! /bin/bash

set -euo pipefail

CHANNEL="$(echo "$RUSTUP_TOOLCHAIN" | sed -n 's/^bisector-\(nightly-[0-9]\{4\}-[0-9]\{2\}-[0-9]\{2\}\)-.*$/\1/;T;p')"

unset RUSTUP_TOOLCHAIN

if [[ -z "$CHANNEL" ]]; then
    if [[ ! -f first_commit_seen ]]; then
        touch first_commit_seen
        exit 1
    else
        exit 0
    fi
fi

# smoelius: The rust-toolchain file should reflect the most recent successful build. So if the
# current build fails, restore the rust-toolchain file's previous contents.
TMP="$(mktemp -p . -t rust-toolchain.XXXXXXXXXX)"
cp rust-toolchain "$TMP"
trap '
    STATUS="$?"
    if [[ "$STATUS" -eq 0 ]]; then
        rm "$TMP"
    else
        mv "$TMP" rust-toolchain
    fi
    exit "$STATUS"
' EXIT

sed -i "s/^channel = \"[^\"]*\"$/channel = \"$CHANNEL\"/" rust-toolchain

if [[ ! -f first_channel_seen ]]; then
    cargo build --tests || (touch first_channel_seen && false)
elif [[ ! -f successful_build_seen ]]; then
    # smoelius: Pretend the build succeeds (even if it doesn't) and proceed backwards until we find
    # one that actually succeeds.
    (cargo build --tests && touch successful_build_seen) || true
else
    cargo build --tests
fi
"#;

const TEMPORARY_FILES: &[&str] = &[
    "first_channel_seen",
    "first_commit_seen",
    "successful_build_seen",
];

pub fn bisect(opts: &Dylint, path: &Path, start: &str) -> Result<()> {
    Command::new("cargo")
        .args(&["bisect-rustc", "-V"])
        .success()
        .with_context(|| "Could not find `cargo-bisect-rustc`. Is it installed?")?;

    let script = script()?;

    let test_dir = path
        .canonicalize()
        .with_context(|| format!("Could not canonicalize {:?}", path))?;

    remove_temporary_files(path);

    let mut command = Command::new("cargo");
    command.args(&[
        "bisect-rustc",
        "--start",
        start,
        "--preserve",
        "--regress=success",
        "--script",
        &*script.path().to_string_lossy(),
        "--test-dir",
        &*test_dir.to_string_lossy(),
    ]);
    if opts.quiet {
        command.stderr(Stdio::null());
    }
    // smoelius: `cargo-bisect-rustc`'s progress bars are displayed on `stdout`.
    if opts.quiet || !atty::is(Stream::Stdout) {
        command.stdout(Stdio::null());
    }
    let result = command.success();

    remove_temporary_files(path);

    result?;

    // smoelius: The build might not actually have succeeded. (See the note above about proceeding
    // backwards.) So verify that the build succeeded before returning `Ok(())`.
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow!("Could not get file name"))?;
    let description = format!("`{}`", file_name.to_string_lossy());
    dylint_internal::build(&description, opts.quiet)
        .sanitize_environment()
        .current_dir(path)
        .args(&["--tests"])
        .success()
}

fn script() -> Result<NamedTempFile> {
    let tempfile_unopened =
        NamedTempFile::new_in(".").with_context(|| "Could not create named temp file")?;
    {
        let mut tempfile_opened =
            NamedTempFile::new_in(".").with_context(|| "Could not create named temp file")?;
        tempfile_opened
            .write_all(SCRIPT.as_bytes())
            .with_context(|| format!("Could not write to {:?}", tempfile_opened.path()))?;

        let metadata = tempfile_opened
            .as_file()
            .metadata()
            .with_context(|| format!("Could not get metadata of {:?}", tempfile_opened.path()))?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        tempfile_opened
            .as_file()
            .set_permissions(permissions)
            .with_context(|| {
                format!("Could not set permissions of {:?}", tempfile_opened.path())
            })?;

        rename(tempfile_opened.path(), tempfile_unopened.path()).with_context(|| {
            format!(
                "Could not rename {:?} to {:?}",
                tempfile_opened.path(),
                tempfile_unopened.path()
            )
        })?;
    }
    Ok(tempfile_unopened)
}

fn remove_temporary_files(path: &Path) {
    for temporary_file in TEMPORARY_FILES {
        remove_file(path.join(temporary_file)).unwrap_or_default();
    }
}
