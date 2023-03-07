use anyhow::Result;
use assert_cmd::Command;
use cargo_metadata::{Dependency, Metadata, MetadataCommand};
use dylint_internal::cargo::current_metadata;
use lazy_static::lazy_static;
use regex::Regex;
use sedregex::find_and_replace;
use semver::Version;
use std::{ffi::OsStr, fs::read_to_string, path::Path};
use tempfile::tempdir;
use test_log::test;

lazy_static! {
    static ref METADATA: Metadata = current_metadata().unwrap();
}

#[test]
fn versions_are_equal() {
    for package in &METADATA.packages {
        assert_eq!(
            package.version.to_string(),
            env!("CARGO_PKG_VERSION"),
            "{}",
            package.name
        );
    }
}

#[test]
fn nightly_crates_have_same_version_as_workspace() {
    for path in ["../driver", "../utils/linting"] {
        let metadata = MetadataCommand::new()
            .current_dir(path)
            .no_deps()
            .exec()
            .unwrap();
        let package = metadata.root_package().unwrap();
        assert_eq!(package.version.to_string(), env!("CARGO_PKG_VERSION"));
    }
}

#[test]
fn versions_are_exact_and_match() {
    for package in &METADATA.packages {
        for Dependency { name: dep, req, .. } in &package.dependencies {
            if dep.starts_with("dylint") {
                assert!(
                    req.to_string().starts_with('='),
                    "`{}` dependency on `{dep}` is not exact",
                    package.name
                );
                assert!(
                    req.matches(&Version::parse(env!("CARGO_PKG_VERSION")).unwrap()),
                    "`{}` dependency on `{dep}` does not match `{}`",
                    package.name,
                    env!("CARGO_PKG_VERSION"),
                );
            }
        }
    }
}

#[test]
fn requirements_do_not_include_patch_versions() {
    let metadata = ["../driver", "../utils/linting"].map(|path| {
        MetadataCommand::new()
            .current_dir(path)
            .no_deps()
            .exec()
            .unwrap()
    });

    for metadata in std::iter::once(&*METADATA).chain(metadata.iter()) {
        for package in &metadata.packages {
            for Dependency { name: dep, req, .. } in &package.dependencies {
                if dep.starts_with("dylint") {
                    continue;
                }
                assert!(
                    req.comparators
                        .iter()
                        .all(|comparator| comparator.patch.is_none()),
                    "`{}` requirement on `{dep}` includes patch version: {req}",
                    package.name
                );
            }
        }
    }
}

#[test]
fn workspace_and_cargo_dylint_readmes_are_equivalent() {
    let workspace_readme = readme_contents(".").unwrap();

    let cargo_dylint_readme = readme_contents("cargo-dylint").unwrap();

    let lifted_cargo_dylint_readme = find_and_replace(
        &cargo_dylint_readme,
        &[r#"s/(?m)^(\[[^\]]*\]: *\.)\./${1}/g"#],
    )
    .unwrap();

    compare_lines(&workspace_readme, &lifted_cargo_dylint_readme);
}

#[test]
fn cargo_dylint_and_dylint_readmes_are_equal() {
    let cargo_dylint_readme = readme_contents("cargo-dylint").unwrap();

    let dylint_readme = readme_contents("dylint").unwrap();

    compare_lines(&cargo_dylint_readme, &dylint_readme);
}

#[test]
fn readmes_do_not_use_inline_links() {
    for entry in walkdir(false) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.file_name() != Some(OsStr::new("README.md")) {
            continue;
        }
        let readme = read_to_string(path).unwrap();
        assert!(
            !Regex::new(r#"\[[^\]]*\]\("#).unwrap().is_match(&readme),
            "`{}` uses inline links",
            path.canonicalize().unwrap().to_string_lossy()
        );
    }
}

#[test]
fn readme_reference_links_are_sorted() {
    let re = Regex::new(r#"^\[[^\]]*\]:"#).unwrap();
    for entry in walkdir(true) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.file_name() != Some(OsStr::new("README.md")) {
            continue;
        }
        let readme = read_to_string(path).unwrap();
        let links = readme
            .lines()
            .filter(|line| re.is_match(line))
            .collect::<Vec<_>>();
        let mut links_sorted = links.clone();
        links_sorted.sort_unstable();
        assert!(
            links_sorted == links,
            "contents of `{}` are not what was expected:\n{}\n",
            path.canonicalize().unwrap().to_string_lossy(),
            links_sorted.join("\n")
        );
    }
}

#[cfg(not(target_os = "windows"))]
#[test]
fn markdown_link_check() {
    let tempdir = tempdir().unwrap();

    Command::new("npm")
        .args(["install", "markdown-link-check"])
        .current_dir(&tempdir)
        .assert()
        .success();

    // smoelius: https://github.com/rust-lang/crates.io/issues/788
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/markdown_link_check.json");

    for entry in walkdir(true) {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension() != Some(OsStr::new("md")) {
            continue;
        }

        // smoelius: Skip `dylint_linting` until I can get it to build on `docs.rs`.
        if path.ends_with("utils/linting/README.md") {
            continue;
        }

        Command::new("npx")
            .args([
                "markdown-link-check",
                "--config",
                &config.to_string_lossy(),
                &path.to_string_lossy(),
            ])
            .current_dir(&tempdir)
            .assert()
            .success();
    }
}

fn readme_contents(dir: impl AsRef<Path>) -> Result<String> {
    #[allow(unknown_lints, env_cargo_path)]
    read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(dir)
            .join("README.md"),
    )
    .map_err(Into::into)
}

fn compare_lines(left: &str, right: &str) {
    assert_eq!(left.lines().count(), right.lines().count());

    for (left, right) in left.lines().zip(right.lines()) {
        assert_eq!(left, right);
    }
}

// smoelius: Skip examples directory for now.
fn walkdir(include_examples: bool) -> impl Iterator<Item = walkdir::Result<walkdir::DirEntry>> {
    #[allow(unknown_lints, env_cargo_path)]
    walkdir::WalkDir::new(Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))
        .into_iter()
        .filter_entry(move |entry| {
            entry.path().file_name() != Some(OsStr::new("target"))
                && (include_examples || entry.path().file_name() != Some(OsStr::new("examples")))
        })
}
