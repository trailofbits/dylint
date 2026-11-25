use assert_cmd::cargo::cargo_bin_cmd;
use dylint_internal::{
    CommandExt, cargo, env, library_filename_with_toolchain, library_plain_filename,
    packaging::isolate,
    rustup::active_toolchain,
    testing::{copy_dir, new_template},
};
use predicates::prelude::*;
use std::{
    fs::{OpenOptions, read_dir, read_to_string, write},
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::{tempdir, tempdir_in};

// smoelius: "Separate lints into categories" commit
const REV: &str = "402fc24351c60a3c474e786fd76aa66aa8638d55";

#[test]
fn array_pattern() {
    let assert = cargo_bin_cmd!("cargo-dylint")
        .current_dir("../fixtures/array_pattern")
        .args(["dylint", "list"])
        .assert()
        .success();
    let stdout = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(2, lines.len());
    assert!(lines[0].starts_with("clippy "));
    assert!(lines[1].starts_with("question_mark_in_expression "));
}

#[cfg(unix)]
#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn edition_2021() {
    use assert_cmd::assert::OutputAssertExt;
    use dylint_internal::rustup::SanitizeEnvironment;
    use std::{fs::create_dir, os::unix::fs::symlink, path::Path};

    let cargo_home = tempdir().unwrap();
    let cargo_home_bin = cargo_home.path().join("bin");
    create_dir(&cargo_home_bin).unwrap();
    let cargo_dylint = env!("CARGO_BIN_EXE_cargo-dylint");
    symlink(cargo_dylint, cargo_home_bin.join("cargo-dylint")).unwrap();

    // smoelius: Sanity. Because this test is likely to have been run by `cargo test`, the
    // environment variables `CARGO`, etc. will have already been set. Thus, we must call
    // `sanitize_environment` to clear them.
    std::process::Command::new("rustup")
        .sanitize_environment()
        .current_dir("../fixtures/edition_2021")
        .args(["which", "cargo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1.84"));

    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../examples/general/crate_wide_allow"
    ));

    // smoelius: The next command must be `cargo` so that we invoke the `rustup` proxy. The command
    // cannot be `cargo-dylint`.
    std::process::Command::new("cargo")
        .sanitize_environment()
        .current_dir("../fixtures/edition_2021")
        .env("CARGO_HOME", cargo_home.path())
        .args(["dylint", "--path", &path.to_string_lossy()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("2024").not());
}

#[test]
fn invalid_pattern() {
    for pattern in ["/*", "../*"] {
        let tempdir = tempdir().unwrap();

        cargo::init("package `invalid_pattern_test`")
            .build()
            .current_dir(&tempdir)
            .args(["--name", "invalid_pattern_test"])
            .success()
            .unwrap();

        let mut file = OpenOptions::new()
            .append(true)
            .open(tempdir.path().join("Cargo.toml"))
            .unwrap();

        // smoelius: For the `../*` test to be effective, there must be multiple copies of Dylint in
        // Cargo's `checkouts` directory.
        write!(
            file,
            r#"
[workspace.metadata.dylint]
libraries = [
    {{ git = "https://github.com/trailofbits/dylint", pattern = "examples/general/crate_wide_allow", rev = "{REV}" }},
    {{ git = "https://github.com/trailofbits/dylint", pattern = "{pattern}" }},
]
"#,
        )
        .unwrap();

        cargo_bin_cmd!("cargo-dylint")
            .current_dir(&tempdir)
            .args(["dylint", "--all"])
            .assert()
            .failure()
            .stderr(
                predicate::str::is_match(r#"Could not canonicalize "[^"]*""#)
                    .unwrap()
                    .or(predicate::str::is_match(
                        r"Pattern `[^`]*` could refer to `[^`]*`, which is outside of `[^`]*`",
                    )
                    .unwrap()),
            );
    }
}

#[test]
fn library_packages_in_dylint_toml() {
    let temp_fixture = tempdir_in("../fixtures").unwrap();
    copy_dir(
        Path::new("../fixtures/library_packages_in_dylint_toml"),
        temp_fixture.path(),
    )
    .unwrap();
    let target_directory = temp_fixture.path().join("target");

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&temp_fixture)
        .env(env::CARGO_TARGET_DIR, &target_directory)
        .env_remove(env::DYLINT_LIBRARIES_DIR)
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "\nwarning: `unwrap`s that could be combined\n",
        ));

    let toolchain =
        active_toolchain(Path::new("../examples/restriction/collapsible_unwrap")).unwrap();
    let release_directory = library_release_directory(
        &target_directory.join("dylint/libraries"),
        "collapsible_unwrap",
        &toolchain,
    );
    assert!(
        release_directory
            .join(library_filename_with_toolchain(
                "collapsible_unwrap",
                &toolchain,
            ))
            .is_file()
    );
}

#[test]
fn library_packages_with_configured_build_directory_and_dylint_link() {
    let temp_fixture = tempdir_in("../fixtures").unwrap();
    copy_dir(
        Path::new("../fixtures/library_packages_in_dylint_toml"),
        temp_fixture.path(),
    )
    .unwrap();
    let build_directory = tempdir().unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&temp_fixture)
        .env(env::CARGO_BUILD_BUILD_DIR, build_directory.path())
        .env(env::DYLINT_LINK_ENABLE_COPY_LIBRARY, "1")
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "\nwarning: `unwrap`s that could be combined\n",
        ));
}

#[test]
fn libraries_dir_shared_across_workspaces() {
    libraries_dir_shared_across_workspaces_inner(false);
}

#[test]
fn libraries_dir_with_configured_build_directory_and_dylint_link() {
    libraries_dir_shared_across_workspaces_inner(true);
}

fn libraries_dir_shared_across_workspaces_inner(copy_with_dylint_link: bool) {
    let libraries_directory = tempdir().unwrap();
    let build_directory = tempdir().unwrap();
    let toolchain =
        active_toolchain(Path::new("../examples/restriction/collapsible_unwrap")).unwrap();
    for first_build in [true, false] {
        let fixture = tempdir_in("../fixtures").unwrap();
        copy_dir(
            Path::new("../fixtures/library_packages_in_dylint_toml"),
            fixture.path(),
        )
        .unwrap();
        let target_directory = fixture.path().join("target");

        let mut command = cargo_bin_cmd!("cargo-dylint");
        command
            .current_dir(&fixture)
            .env(env::CARGO_TARGET_DIR, &target_directory)
            .env(env::DYLINT_LIBRARIES_DIR, libraries_directory.path())
            .env(
                env::DYLINT_LINK_ENABLE_COPY_LIBRARY,
                if copy_with_dylint_link { "1" } else { "0" },
            )
            .env_remove(env::CARGO_BUILD_BUILD_DIR);
        if copy_with_dylint_link {
            command.env(env::CARGO_BUILD_BUILD_DIR, build_directory.path());
        }
        let assert =
            command
                .args(["dylint", "--all"])
                .assert()
                .success()
                .stderr(predicate::str::contains(
                    "\nwarning: `unwrap`s that could be combined\n",
                ));
        if first_build {
            assert.stderr(predicate::str::contains("Compiling collapsible_unwrap"));
        } else {
            assert.stderr(predicate::str::contains("Compiling").not());
        }

        assert!(!target_directory.join("dylint/libraries").exists());

        let release_directory =
            library_release_directory(libraries_directory.path(), "collapsible_unwrap", &toolchain);
        assert!(
            release_directory
                .join(library_filename_with_toolchain(
                    "collapsible_unwrap",
                    &toolchain,
                ))
                .is_file()
        );
        assert!(
            release_directory
                .join(library_plain_filename("collapsible_unwrap"))
                .is_file()
        );
    }
}

fn library_release_directory(root: &Path, package: &str, toolchain: &str) -> PathBuf {
    let partitions = read_dir(root.join(package))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(1, partitions.len());
    let key = partitions[0].file_name().unwrap().to_string_lossy();
    assert!(
        key.starts_with(concat!(env!("CARGO_PKG_VERSION"), "-")),
        "unexpected cache key: {key}"
    );
    let release_directory = partitions[0].join(toolchain).join("release");
    assert!(release_directory.is_dir());
    release_directory
}

#[test]
#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
fn libraries_dir_separates_git_revisions() {
    libraries_dir_separates_git_revisions_inner(false);
}

#[test]
#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
fn libraries_dir_separates_git_revisions_with_dylint_link() {
    libraries_dir_separates_git_revisions_inner(true);
}

fn libraries_dir_separates_git_revisions_inner(copy_with_dylint_link: bool) {
    let library = tempdir().unwrap();
    new_template(library.path()).unwrap();
    let repository = git2::Repository::init(&library).unwrap();
    let first = commit_library(&repository);
    repository
        .tag_lightweight(
            "first",
            &repository.find_object(first, None).unwrap(),
            false,
        )
        .unwrap();
    let source = library.path().join("src/lib.rs");
    let contents = read_to_string(&source).unwrap();
    write(
        &source,
        contents.replace("description goes here", "second revision"),
    )
    .unwrap();
    let second = commit_library(&repository);
    let url = url::Url::from_directory_path(dunce::canonicalize(&library).unwrap())
        .unwrap()
        .to_string();
    let libraries_directory = tempdir().unwrap();
    let build_directory = tempdir().unwrap();
    let toolchain = active_toolchain(library.path()).unwrap();

    for (flag, reference, description, fresh) in [
        ("--rev", first.to_string(), "description goes here", true),
        ("--tag", "first".to_owned(), "description goes here", false),
        ("--rev", second.to_string(), "second revision", true),
        ("--rev", first.to_string(), "description goes here", false),
    ] {
        let consumer = tempdir_in("../fixtures").unwrap();
        copy_dir(
            Path::new("../fixtures/library_packages_in_dylint_toml"),
            consumer.path(),
        )
        .unwrap();
        let command = || {
            let mut command = cargo_bin_cmd!("cargo-dylint");
            command
                .current_dir(&consumer)
                .env(env::CARGO_TARGET_DIR, consumer.path().join("target"))
                .env(env::DYLINT_LIBRARIES_DIR, libraries_directory.path())
                .env(
                    env::DYLINT_LINK_ENABLE_COPY_LIBRARY,
                    if copy_with_dylint_link { "1" } else { "0" },
                )
                .env_remove(env::CARGO_BUILD_BUILD_DIR)
                .args(["dylint", "list", "--git", &url, flag, &reference]);
            if copy_with_dylint_link {
                command.env(env::CARGO_BUILD_BUILD_DIR, build_directory.path());
            }
            command
        };
        let assert = command()
            .assert()
            .success()
            .stdout(predicate::str::contains(description));
        if !fresh {
            assert.stderr(predicate::str::contains("Compiling").not());
        }
        command()
            .arg("--no-build")
            .assert()
            .success()
            .stdout(predicate::str::contains(description));
    }

    let partitions = read_dir(libraries_directory.path().join("fill_me_in"))
        .unwrap()
        .filter(|entry| entry.as_ref().unwrap().path().join(&toolchain).is_dir())
        .count();
    assert_eq!(2, partitions);
}

fn commit_library(repository: &git2::Repository) -> git2::Oid {
    let mut index = repository.index().unwrap();
    index
        .add_all(["."], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::now("Dylint tests", "dylint@example.invalid").unwrap();
    let parent = repository
        .head()
        .ok()
        .map(|head| head.peel_to_commit().unwrap());
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Test library revision",
            &tree,
            &parent.iter().collect::<Vec<_>>(),
        )
        .unwrap()
}

#[test]
fn libraries_dir_must_be_absolute() {
    for path in ["", "relative/path"] {
        cargo_bin_cmd!("cargo-dylint")
            .current_dir("../fixtures/library_packages_in_dylint_toml")
            .env(env::DYLINT_LIBRARIES_DIR, path)
            .args(["dylint", "list"])
            .assert()
            .failure()
            .stderr(predicate::str::contains(format!(
                "DYLINT_LIBRARIES_DIR contains `{path}`, which is not absolute"
            )));
    }
}

#[test]
fn library_packages_with_rust_toolchain() {
    let tempdir = tempdir().unwrap();

    let assert = cargo_bin_cmd!("cargo-dylint")
        .current_dir("../fixtures/library_packages_with_rust_toolchain")
        .env(env::RUST_LOG, "debug")
        // smoelius: Use a temporary target directory to shorten build directory paths for Windows.
        .env(env::CARGO_TARGET_DIR, &*tempdir.path().to_string_lossy())
        .args(["dylint", "--all"])
        .assert()
        .success();

    if cfg!(all(
        feature = "cargo-cli",
        target_arch = "x86_64",
        not(target_os = "windows")
    )) {
        assert.stderr(predicate::str::contains(
            r#"/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo" "fetch""#,
        ));
    }
}

#[test]
fn list() {
    let tempdir = tempdir().unwrap();

    cargo::init("package `list_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "list_test"])
        .success()
        .unwrap();

    let mut file = OpenOptions::new()
        .append(true)
        .open(tempdir.path().join("Cargo.toml"))
        .unwrap();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/trailofbits/dylint"
pattern = "examples/general/crate_wide_allow"
"#,
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<unbuilt>"));
}

/// Verify that changes to workspace metadata cause the lints to be rerun.
#[test]
fn metadata_change() {
    let tempdir = tempdir().unwrap();

    cargo::init("package `metadata_change_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "metadata_change_test"])
        .success()
        .unwrap();

    isolate(tempdir.path()).unwrap();

    let mut file = OpenOptions::new()
        .append(true)
        .open(tempdir.path().join("Cargo.toml"))
        .unwrap();

    let path_spec = toml::Value::String(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/general/crate_wide_allow"
        )
        .to_owned(),
    );

    write!(
        file,
        r"
    [[workspace.metadata.dylint.libraries]]
    path = {path_spec}
    "
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Checking metadata_change_test"));

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Checking metadata_change_test").not());

    let path_spec = toml::Value::String(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/restriction/question_mark_in_expression"
        )
        .to_owned(),
    );

    write!(
        file,
        r"
        [[workspace.metadata.dylint.libraries]]
        path = {path_spec}
        "
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Checking metadata_change_test"));
}

#[test]
fn multiple_patterns() {
    cargo_bin_cmd!("cargo-dylint")
        .args([
            "dylint",
            "list",
            "--path",
            "../examples/general",
            "--pattern",
            "abs_home_path",
            "--pattern",
            "crate_wide_allow",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("abs_home_path")
                .and(predicate::str::contains("crate_wide_allow")),
        );
}

#[test]
fn no_metadata_with_path() {
    cargo_bin_cmd!("cargo-dylint")
        .args([
            "dylint",
            "list",
            "--no-metadata",
            "--path",
            "../examples/general",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "`--git` or `--path` cannot be used with `--no-metadata`",
        ));
}

#[test]
fn nonexistent_git_library() {
    let tempdir = tempdir().unwrap();

    cargo::init("package `nonexistent_git_library_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "nonexistent_git_library_test"])
        .success()
        .unwrap();

    let mut file = OpenOptions::new()
        .append(true)
        .open(tempdir.path().join("Cargo.toml"))
        .unwrap();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/trailofbits/dylint"
pattern = "examples/general/crate_wide_allow"
"#
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/trailofbits/dylint"
pattern = "examples/general/nonexistent_library"
"#
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No paths matched"));
}

#[test]
fn nonexistent_path_library() {
    let tempdir = tempdir().unwrap();

    cargo::init("package `nonexistent_path_library_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "nonexistent_path_library_test"])
        .success()
        .unwrap();

    isolate(tempdir.path()).unwrap();

    let mut file = OpenOptions::new()
        .append(true)
        .open(tempdir.path().join("Cargo.toml"))
        .unwrap();

    let path_spec = toml::Value::String(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/general/crate_wide_allow"
        )
        .to_owned(),
    );

    write!(
        file,
        r"
[[workspace.metadata.dylint.libraries]]
path = {path_spec}
"
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success();

    let path_spec = toml::Value::String(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/general/nonexistent_library"
        )
        .to_owned(),
    );

    write!(
        file,
        r"
[[workspace.metadata.dylint.libraries]]
path = {path_spec}
"
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No library packages found in"));
}

/// Verify that changes to `RUSTFLAGS` do not cause workspace metadata entries to be rebuilt.
#[test]
fn rustflags_change() {
    let tempdir = tempdir().unwrap();

    cargo::init("package `rustflags_change_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "rustflags_change_test"])
        .success()
        .unwrap();

    isolate(tempdir.path()).unwrap();

    let mut file = OpenOptions::new()
        .append(true)
        .open(tempdir.path().join("Cargo.toml"))
        .unwrap();

    let path_spec = toml::Value::String(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/general/crate_wide_allow"
        )
        .to_owned(),
    );

    write!(
        file,
        r"
[[workspace.metadata.dylint.libraries]]
path = {path_spec}
"
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Compiling"));

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .env(env::RUSTFLAGS, "--verbose")
        .args(["dylint", "--all"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Compiling").not());
}

#[test]
fn unknown_keys() {
    let tempdir = tempdir().unwrap();

    cargo::init("package `unknown_keys_test`")
        .build()
        .current_dir(&tempdir)
        .args(["--name", "unknown_keys_test"])
        .success()
        .unwrap();

    let mut file = OpenOptions::new()
        .append(true)
        .open(tempdir.path().join("Cargo.toml"))
        .unwrap();

    write!(
        file,
        r#"
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/trailofbits/dylint"
pattern = "examples/general/crate_wide_allow"
"#,
    )
    .unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .success();

    writeln!(file, r#"revision = "{REV}""#).unwrap();

    cargo_bin_cmd!("cargo-dylint")
        .current_dir(&tempdir)
        .args(["dylint", "--all"])
        .assert()
        .failure()
        .stderr(predicate::str::is_match(r"Unknown library keys:\r?\n\s*revision\r?\n").unwrap());
}
