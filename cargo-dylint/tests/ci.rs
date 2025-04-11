#![cfg(not(coverage))]

use anyhow::Result;
use assert_cmd::Command;
use cargo_metadata::{Dependency, Metadata, MetadataCommand};
use dylint_internal::{cargo::current_metadata, env, examples, home};
use regex::Regex;
use semver::{Op, Version};
use similar_asserts::SimpleDiff;
use std::{
    env::{set_current_dir, set_var, var},
    ffi::OsStr,
    fmt::Write as _,
    fs::{read_dir, read_to_string, write},
    io::{Write as _, stderr},
    path::{Component, Path},
    sync::{LazyLock, Mutex},
};

static METADATA: LazyLock<Metadata> = LazyLock::new(|| current_metadata().unwrap());

#[ctor::ctor]
fn initialize() {
    set_current_dir("..").unwrap();
    set_var(env::CARGO_TERM_COLOR, "never");
}

#[test]
fn actionlint() {
    Command::new("go")
        .args([
            "install",
            "github.com/rhysd/actionlint/cmd/actionlint@latest",
        ])
        .assert()
        .success();
    let home = home::home_dir().unwrap();
    Command::new(home.join("go/bin/actionlint"))
        .assert()
        .success();
}

#[test]
fn versions_are_equal() {
    for package in &METADATA.packages {
        assert_eq!(
            env!("CARGO_PKG_VERSION"),
            package.version.to_string(),
            "{}",
            package.name
        );
    }
}

#[test]
fn nightly_crates_have_same_version_as_workspace() {
    for path in ["driver", "utils/linting"] {
        let metadata = MetadataCommand::new()
            .current_dir(path)
            .no_deps()
            .exec()
            .unwrap();
        let package = metadata.root_package().unwrap();
        assert_eq!(env!("CARGO_PKG_VERSION"), package.version.to_string());
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
fn patch_version_requirements_are_exact() {
    let metadata = ["driver", "utils/linting"].map(|path| {
        MetadataCommand::new()
            .current_dir(path)
            .no_deps()
            .exec()
            .unwrap()
    });

    for metadata in std::iter::once(&*METADATA).chain(metadata.iter()) {
        for package in &metadata.packages {
            for Dependency { name: dep, req, .. } in &package.dependencies {
                assert!(
                    req.comparators
                        .iter()
                        .all(|comparator| comparator.op == Op::Exact || comparator.patch.is_none()),
                    "`{}` requirement on `{dep}` includes patch version and is not exact: {req}",
                    package.name
                );
            }
        }
    }
}

#[test]
fn workspace_and_cargo_dylint_readmes_are_equivalent() {
    let re = Regex::new(r"(?m)^(\[[^\]]*\]: *\.)\.").unwrap();

    let workspace_readme = readme_contents(".").unwrap();

    let cargo_dylint_readme = readme_contents("cargo-dylint").unwrap();

    let lifted_cargo_dylint_readme = re.replace_all(&cargo_dylint_readme, "${1}");

    compare_lines(&workspace_readme, &lifted_cargo_dylint_readme);
}

#[test]
fn cargo_dylint_and_dylint_readmes_are_equal() {
    let cargo_dylint_readme = readme_contents("cargo-dylint").unwrap();

    let dylint_readme = readme_contents("dylint").unwrap();

    compare_lines(&cargo_dylint_readme, &dylint_readme);
}

#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn format_example_readmes() {
    let re = Regex::new(r"(?m)^\s*///\s?(.*)$").unwrap();

    for result in examples::iter(false).unwrap() {
        let example_dir = result.unwrap();

        assert!(example_dir.is_dir());

        let src_dir = example_dir.join("src");

        if !src_dir.try_exists().unwrap() {
            continue;
        }

        let mut readme_lines = Vec::new();
        for entry in read_dir(src_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension() != Some(OsStr::new("rs")) {
                continue;
            }
            let contents = read_to_string(path).unwrap();
            let lines = contents
                .lines()
                .skip_while(|line| !line.ends_with("_lint! {"))
                .skip(1)
                .take_while(|&line| line != "}");
            for line in lines {
                let Some(captures) = re.captures(line) else {
                    continue;
                };
                assert_eq!(2, captures.len());
                let readme_line = captures.get(1).unwrap().as_str();
                if readme_line.starts_with("# ") {
                    continue;
                }
                readme_lines.push(readme_line.to_owned());
            }
        }
        if readme_lines.is_empty() {
            continue;
        }

        let mut readme = String::new();
        for line in [
            format!("# {}", example_dir.file_name().unwrap().to_string_lossy()),
            String::new(),
        ]
        .into_iter()
        .chain(readme_lines)
        {
            writeln!(readme, "{line}").unwrap();
        }

        let readme_path = example_dir.join("README.md");

        if env::enabled("BLESS") {
            write(readme_path, readme).unwrap();
        } else {
            let readme_expected = read_to_string(&readme_path).unwrap();
            assert!(
                readme_expected == readme,
                "{}",
                SimpleDiff::from_str(&readme_expected, &readme, "left", "right")
            );
        }
    }
}

#[test]
fn format_util_readmes() {
    preserves_cleanliness("format_util_readmes", false, || {
        for entry in read_dir("utils").unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            Command::new("cargo")
                .arg("rdme")
                .current_dir(path)
                .assert()
                .success();
        }
    });
}

#[test]
fn hack_feature_powerset_udeps() {
    Command::new("rustup")
        // smoelius: `--check-cfg cfg(test)` to work around the following issue:
        // https://github.com/est31/cargo-udeps/issues/293
        .env(env::RUSTFLAGS, "-D warnings --check-cfg cfg(test)")
        .args([
            "run",
            "nightly",
            "cargo",
            "hack",
            "--feature-powerset",
            "udeps",
        ])
        .assert()
        .success();
}

#[test]
fn license() {
    let re = Regex::new(r"^[^:]*\b(Apache|BSD-3-Clause|ISC|MIT|N/A)\b").unwrap();

    // smoelius: Skip examples directory for now.
    for entry in walkdir(false).with_file_name("Cargo.toml") {
        let entry = entry.unwrap();
        let path = entry.path();
        for line in std::str::from_utf8(
            &Command::new("cargo")
                .args(["license", "--manifest-path", &path.to_string_lossy()])
                .assert()
                .success()
                .get_output()
                .stdout,
        )
        .unwrap()
        .lines()
        {
            // smoelius: Exception for Cargo dependencies.
            if line == "MPL-2.0+ (3): bitmaps, im-rc, sized-chunks" {
                continue;
            }
            // smoelius: Exception for `idna` dependencies.
            if line
                == "Unicode-3.0 (19): icu_collections, icu_locid, icu_locid_transform, \
                    icu_locid_transform_data, icu_normalizer, icu_normalizer_data, icu_properties, \
                    icu_properties_data, icu_provider, icu_provider_macros, litemap, tinystr, \
                    writeable, yoke, yoke-derive, zerofrom, zerofrom-derive, zerovec, \
                    zerovec-derive"
            {
                continue;
            }
            // smoelius: Good explanation of the differences between the BSD-3-Clause and MIT
            // licenses: https://opensource.stackexchange.com/a/582
            assert!(re.is_match(line), "{line:?} does not match");
        }
    }
}

#[test]
fn markdown_does_not_use_inline_links() {
    let re = Regex::new(r"\[[^\]]*\]\(").unwrap();
    // smoelius: Skip examples directory for now.
    for entry in walkdir(false).with_extension("md") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.file_name() == Some(OsStr::new("CHANGELOG.md")) {
            continue;
        }
        let markdown = read_to_string(path).unwrap();
        assert!(
            !re.is_match(&markdown),
            "`{}` uses inline links",
            path.canonicalize().unwrap().to_string_lossy()
        );
    }
}

#[test]
fn markdown_reference_links_are_sorted() {
    let re = Regex::new(r"^\[[^\]]*\]:").unwrap();
    for entry in walkdir(true).with_extension("md") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.file_name() == Some(OsStr::new("CHANGELOG.md")) {
            continue;
        }
        let markdown = read_to_string(path).unwrap();
        let links = markdown
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

#[test]
fn markdown_reference_links_are_valid_and_used() {
    const CODE: &str = "`[^`]*`";
    const CODE_BLOCK: &str = "```([^`]|`[^`]|``[^`])*```";
    let ref_re = Regex::new(&format!(r"(?m){CODE}|{CODE_BLOCK}|\[([^\]]*)\]([^:]|$)")).unwrap();
    let link_re = Regex::new(r"(?m)^\[([^\]]*)\]:").unwrap();
    for entry in walkdir(true).with_extension("md") {
        let entry = entry.unwrap();
        let path = entry.path();
        // smoelius: The ` ["\n```"] ` in `missing_doc_comment_openai`'s readme causes problems, and
        // I haven't found a good solution/workaround.
        if path.file_name() == Some(OsStr::new("CHANGELOG.md"))
            || path.ends_with("examples/README.md")
            || path
                .components()
                .any(|c| c == Component::Normal(OsStr::new("missing_doc_comment_openai")))
        {
            continue;
        }
        let markdown = read_to_string(path).unwrap();
        let mut refs = ref_re
            .captures_iter(&markdown)
            .filter_map(|captures| {
                // smoelius: 2 because 1 is the parenthesized expression in `CODE_BLOCK`.
                captures
                    .get(2)
                    .map(|m| m.as_str().replace('\r', "").replace('\n', " "))
            })
            .collect::<Vec<_>>();

        // smoelius: The use of `to_lowercase` in the next statement is a convenience and should
        // eventually be removed. `prettier` 2.8.2 stopped lowercasing link labels. But as of this
        // writing, the latest version of the Prettier VS Code extension (9.10.4) still appears to
        // use `prettier` 2.8.0.
        // smoelius: The Prettier VS Code extension was updated. The use of `to_lowercase` is no
        // longer necessary.
        //
        // References:
        // - https://github.com/prettier/prettier/pull/13155
        // - https://github.com/prettier/prettier/blob/main/CHANGELOG.md#282
        // - https://github.com/prettier/prettier-vscode/blob/main/CHANGELOG.md#9103
        // - https://github.com/prettier/prettier-vscode/blob/main/CHANGELOG.md#9110
        let mut links = link_re
            .captures_iter(&markdown)
            .map(|captures| captures.get(1).unwrap().as_str())
            .collect::<Vec<_>>();

        refs.sort_unstable();
        refs.dedup();

        links.sort_unstable();
        links.dedup();

        assert_eq!(refs, links, "failed for {path:?}");
    }
}

// smoelius: `markdown_link_check` must use absolute paths because `npx markdown-link-check` is run
// from a temporary directory.
#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn markdown_link_check() {
    let tempdir = tempfile::tempdir().unwrap();

    // smoelius: Pin `markdown-link-check` to version 3.11 until the following issue is resolved:
    // https://github.com/tcort/markdown-link-check/issues/304
    Command::new("npm")
        .args(["install", "markdown-link-check@3.11"])
        .current_dir(&tempdir)
        .assert()
        .success();

    // smoelius: https://github.com/rust-lang/crates.io/issues/788
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/markdown_link_check.json");

    for entry in walkdir(true).with_extension("md") {
        let entry = entry.unwrap();
        let path = entry.path();

        let path_buf = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path);

        let assert = Command::new("npx")
            .args([
                "markdown-link-check",
                "--config",
                &config.to_string_lossy(),
                &path_buf.to_string_lossy(),
            ])
            .current_dir(&tempdir)
            .assert();
        let stdout = std::str::from_utf8(&assert.get_output().stdout).unwrap();

        assert!(
            stdout
                .lines()
                .skip_while(|line| !line.ends_with(" links checked."))
                .skip(1)
                .all(|line| { line.is_empty() || line.ends_with(" → Status: 500") }),
            "{stdout}"
        );
    }
}

#[test]
fn msrv() {
    for package in &METADATA.packages {
        if package.rust_version.is_none() {
            continue;
        }
        let manifest_dir = package.manifest_path.parent().unwrap();
        Command::new("cargo")
            .args([
                "msrv",
                "verify",
                "--",
                "cargo",
                "check",
                "--no-default-features",
            ])
            .current_dir(manifest_dir)
            .assert()
            .success();
    }
}

#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn prettier_all_but_examples_and_template() {
    Command::new("prettier")
        .args([
            "--check",
            "--ignore-path",
            "cargo-dylint/tests/prettier_ignore.txt",
            "**/*.md",
            "**/*.yml",
        ])
        .assert()
        .success();
}

#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn prettier_examples_and_template() {
    preserves_cleanliness("prettier", true, || {
        Command::new("prettier")
            .args(["--write", "examples/**/*.md", "internal/template/**/*.md"])
            .assert()
            .success();
    });
}

#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn rustdoc_prettier() {
    preserves_cleanliness("rustdoc_prettier", false, || {
        Command::new("rustdoc-prettier")
            .args(["./**/*.rs"])
            .assert()
            .success();
    });
}

#[test]
fn fmt() {
    for entry in walkdir(true).with_file_name("Cargo.toml") {
        let entry = entry.unwrap();
        let path = entry.path();
        let parent = path.parent().unwrap();

        Command::new("cargo")
            .args(["+nightly", "fmt", "--check"])
            .current_dir(parent)
            .assert()
            .success();
    }
}

#[test]
fn shellcheck() {
    for entry in read_dir("scripts").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        Command::new("shellcheck")
            .args(["--exclude=SC2002", &path.to_string_lossy()])
            .assert()
            .success();
    }
}

#[test]
fn sort() {
    for entry in walkdir(true).with_file_name("Cargo.toml") {
        let entry = entry.unwrap();
        let path = entry.path();
        let parent = path.parent().unwrap();
        Command::new("cargo")
            .current_dir(parent)
            .args(["sort", "--check", "--grouped"])
            .assert()
            .success();
    }
}

#[test]
fn update() {
    preserves_cleanliness("update", false, || {
        for entry in walkdir(true).with_file_name("Cargo.lock") {
            let entry = entry.unwrap();
            let path = entry.path();
            let manifest_path = path.with_file_name("Cargo.toml");
            Command::new("cargo")
                .args([
                    "update",
                    "--workspace",
                    "--manifest-path",
                    &manifest_path.to_string_lossy(),
                ])
                .assert()
                .success();
        }
    });
}

fn readme_contents(dir: impl AsRef<Path>) -> Result<String> {
    read_to_string(dir.as_ref().join("README.md")).map_err(Into::into)
}

fn compare_lines(left: &str, right: &str) {
    assert_eq!(left.lines().count(), right.lines().count());

    for (left, right) in left.lines().zip(right.lines()) {
        assert_eq!(left, right);
    }
}

fn walkdir(include_examples: bool) -> impl Iterator<Item = walkdir::Result<walkdir::DirEntry>> {
    walkdir::WalkDir::new(".")
        .into_iter()
        .filter_entry(move |entry| {
            entry.path().file_name() != Some(OsStr::new("target"))
                && (include_examples || entry.path().file_name() != Some(OsStr::new("examples")))
        })
}

trait IntoIterExt {
    fn with_extension(
        self,
        extension: impl AsRef<OsStr> + 'static,
    ) -> impl Iterator<Item = walkdir::Result<walkdir::DirEntry>>;
    fn with_file_name(
        self,
        file_name: impl AsRef<OsStr> + 'static,
    ) -> impl Iterator<Item = walkdir::Result<walkdir::DirEntry>>;
}

impl<T: Iterator<Item = walkdir::Result<walkdir::DirEntry>>> IntoIterExt for T {
    fn with_extension(
        self,
        extension: impl AsRef<OsStr> + 'static,
    ) -> impl Iterator<Item = walkdir::Result<walkdir::DirEntry>> {
        self.filter(move |entry| {
            entry.as_ref().map_or(true, |entry| {
                entry.path().extension() == Some(extension.as_ref())
            })
        })
    }
    fn with_file_name(
        self,
        file_name: impl AsRef<OsStr> + 'static,
    ) -> impl Iterator<Item = walkdir::Result<walkdir::DirEntry>> {
        self.filter(move |entry| {
            entry.as_ref().map_or(true, |entry| {
                entry.path().file_name() == Some(file_name.as_ref())
            })
        })
    }
}

static MUTEX: Mutex<()> = Mutex::new(());

fn preserves_cleanliness(test_name: &str, ignore_blank_lines: bool, f: impl FnOnce()) {
    let _lock = MUTEX.lock().unwrap();

    // smoelius: Do not skip tests when running on GitHub.
    if var(env::CI).is_err() && dirty(false).is_some() {
        #[allow(clippy::explicit_write)]
        writeln!(
            stderr(),
            "Skipping `{test_name}` test as repository is dirty"
        )
        .unwrap();
        return;
    }

    f();

    if let Some(stdout) = dirty(ignore_blank_lines) {
        panic!("{}", stdout);
    }

    // smoelius: If the repository is not dirty with `ignore_blank_lines` set to true, but would be
    // dirty otherwise, then restore the repository's contents.
    if ignore_blank_lines && dirty(false).is_some() {
        Command::new("git")
            .args(["checkout", "."])
            .assert()
            .success();
    }
}

fn dirty(ignore_blank_lines: bool) -> Option<String> {
    let mut command = Command::new("git");
    command.arg("diff");
    if ignore_blank_lines {
        command.arg("--ignore-blank-lines");
    }
    let output = command.output().unwrap();

    // smoelius: `--ignore-blank-lines` does not work with `--exit-code`. So instead check whether
    // stdout is empty.
    if output.stdout.is_empty() {
        None
    } else {
        Some(String::from_utf8(output.stdout).unwrap())
    }
}
