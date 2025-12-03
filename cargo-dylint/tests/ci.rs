#![cfg(not(coverage))]

use anyhow::Result;
use assert_cmd::{Command, cargo::cargo_bin_cmd};
use cargo_metadata::{Dependency, Metadata, MetadataCommand};
use dylint_internal::{cargo::current_metadata, env, examples};
use regex::Regex;
use semver::{Op, Version};
use similar_asserts::SimpleDiff;
use std::{
    env::{remove_var, set_current_dir, var},
    ffi::OsStr,
    fmt::Write as _,
    fs::{read_dir, read_to_string, write},
    io::{Write as _, stderr},
    path::{Component, Path, PathBuf},
    sync::LazyLock,
};

static METADATA: LazyLock<Metadata> = LazyLock::new(|| current_metadata().unwrap());

static DESCRIPTION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"description\s*=\s*"([^"]*)""#).unwrap());

#[ctor::ctor]
fn initialize() {
    set_current_dir("..").unwrap();
    unsafe {
        remove_var(env::CARGO_TERM_COLOR);
    }
}

#[test]
fn actionlint() {
    if Command::new("which")
        .arg("actionlint")
        .assert()
        .try_success()
        .is_err()
    {
        #[allow(clippy::explicit_write)]
        writeln!(
            stderr(),
            "Skipping `actionlint` test as `actionlint` is unavailable"
        )
        .unwrap();
        return;
    }
    Command::new("actionlint").assert().success();
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

#[test]
fn examples_readme_contents() {
    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
    let categories = vec![
        "general",
        "supplementary",
        "restriction",
        "experimental",
        "testing",
    ];

    // Read the existing README
    let readme_path = examples_dir.join("README.md");
    let readme_content = read_to_string(&readme_path).unwrap();

    // Generate just the lint description tables
    let expected_tables = generate_lint_tables(&examples_dir, &categories);

    // Extract the current tables section from README using markers
    let actual_tables = extract_between_markers(&readme_content)
        .unwrap_or_else(|| panic!("Lint description markers not found in README.md"));

    // Compare the trimmed generated content with the actual content
    assert_eq!(
        expected_tables.trim(),
        actual_tables,
        "Lint descriptions in README.md do not match expected content"
    );
}

fn generate_lint_tables(examples_dir: &Path, categories: &[&str]) -> String {
    const EXAMPLE_HEADER: &str = "Example";
    const DESC_HEADER: &str = "Description/check";

    let mut content = String::new();

    // Generate the tables for each category
    for category in categories {
        use std::cmp::max;
        use std::fmt::Write;

        // Get the examples for this category
        let examples = collect_examples_from_category(examples_dir, category);

        // Calculate column widths
        let max_example_width = examples
            .iter()
            .map(|(name, _)| format!("[`{name}`](./{category}/{name})").len())
            .max()
            .unwrap_or(0);

        let max_desc_width = examples
            .iter()
            .map(|(_, description)| description.len())
            .max()
            .unwrap_or(0);

        let example_col_width = max(EXAMPLE_HEADER.len(), max_example_width);
        let desc_col_width = max(DESC_HEADER.len(), max_desc_width);

        // Write header
        write!(content, "\n## {}\n\n", capitalize(category)).unwrap();
        #[allow(clippy::uninlined_format_args)]
        writeln!(
            content,
            "| {:<example_col_width$} | {:<desc_col_width$} |",
            EXAMPLE_HEADER, DESC_HEADER
        )
        .unwrap();
        writeln!(
            content,
            "| {:-<example_col_width$} | {:-<desc_col_width$} |",
            "", ""
        )
        .unwrap(); // Separator line

        // Write rows with padding
        for (name, description) in examples {
            let example_link = format!("[`{name}`](./{category}/{name})");
            #[allow(clippy::uninlined_format_args)]
            writeln!(
                content,
                "| {:<example_col_width$} | {:<desc_col_width$} |",
                example_link, description
            )
            .unwrap();
        }
    }

    content
}

fn extract_between_markers(content: &str) -> Option<String> {
    const START_MARKER: &str = "<!-- lint descriptions start -->";
    const END_MARKER: &str = "<!-- lint descriptions end -->";
    let start = content.find(START_MARKER)? + START_MARKER.len();
    let end = content.find(END_MARKER)?;
    Some(content[start..end].trim().to_string())
}

fn collect_examples_from_category(examples_dir: &Path, category: &str) -> Vec<(String, String)> {
    let mut examples = Vec::new();
    let category_dir = examples_dir.join(category);

    for entry in read_dir(&category_dir).unwrap() {
        let entry = entry.unwrap();
        let metadata = entry.metadata().unwrap();
        if metadata.is_dir() {
            let cargo_toml_path = entry.path().join("Cargo.toml");
            if cargo_toml_path.exists()
                && let Some((name, desc)) = extract_name_and_description(&cargo_toml_path)
            {
                examples.push((name, desc));
            }
        }
    }

    // Sort examples by name
    examples.sort_by(|(a, _), (b, _)| a.cmp(b));
    examples
}

fn extract_name_and_description(cargo_toml_path: &Path) -> Option<(String, String)> {
    let content = read_to_string(cargo_toml_path).ok()?;

    // Get the name from the directory
    let name = cargo_toml_path
        .parent()
        .and_then(|path| path.file_name())
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Extract the description using regex
    let description = if let Some(caps) = DESCRIPTION_REGEX.captures(&content) {
        let desc = caps.get(1).unwrap();
        // Format the description like the bash script does
        let desc_str = desc.as_str();
        if let Some(stripped) = desc_str.strip_prefix("A lint to check for ") {
            capitalize(stripped)
        } else {
            desc_str.to_string()
        }
    } else {
        return None;
    };

    Some((name, description))
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut result = first.to_uppercase().collect::<String>();
            result.extend(chars);
            result
        }
    }
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
    for entry in read_dir("utils").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        Command::new("cargo")
            .args(["rdme", "--check"])
            .current_dir(path)
            .assert()
            .success();
    }
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
        // smoelius: Skip the library template.
        if path.parent().and_then(Path::file_name) == Some(OsStr::new("template")) {
            continue;
        }
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
            // smoelius: Exceptions for `idna` dependencies.
            if line
                == "Unicode-3.0 (18): icu_collections, icu_locale_core, icu_normalizer, \
                    icu_normalizer_data, icu_properties, icu_properties_data, icu_provider, \
                    litemap, potential_utf, tinystr, writeable, yoke, yoke-derive, zerofrom, \
                    zerofrom-derive, zerotrie, zerovec, zerovec-derive"
            {
                continue;
            }
            // smoelius: Good explanation of the differences between the BSD-3-Clause and MIT
            // licenses: https://opensource.stackexchange.com/a/582
            assert!(
                re.is_match(line),
                "failed for `{}`\n{line:?} does not match",
                path.display()
            );
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
        if entry.file_name() == "CHANGELOG.md" {
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
        if entry.file_name() == "CHANGELOG.md" {
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
        if entry.file_name() == "CHANGELOG.md"
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

#[cfg_attr(target_os = "windows", ignore)]
#[test]
#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
fn markdown_link_check() {
    // Skip the test if GITHUB_TOKEN is not available
    let Ok(token) = var(env::GITHUB_TOKEN) else {
        eprintln!(
            "Skipping `markdown_link_check` test as {} environment variable is not set",
            env::GITHUB_TOKEN
        );
        eprintln!(
            "To run this test, set the token: {}=your_token cargo test ...",
            env::GITHUB_TOKEN
        );
        return;
    };

    let tempdir = tempfile::tempdir().unwrap();

    Command::new("npm")
        .args(["install", "markdown-link-check"])
        .current_dir(&tempdir)
        .assert()
        .success();

    // smoelius: https://github.com/rust-lang/crates.io/issues/788
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/markdown_link_check.json");

    // Read the original config content
    let mut config_content = read_to_string(&config).unwrap();
    let temp_config = tempdir.path().join("markdown_link_check.json");

    // Replace ${GITHUB_TOKEN} with the actual token
    config_content = config_content.replace("${GITHUB_TOKEN}", &token);
    write(&temp_config, config_content).unwrap();

    for entry in walkdir(true).with_extension("md") {
        let entry = entry.unwrap();
        let path = entry.path();

        let path_buf = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path);

        let mut command = Command::new("npx");
        command.args([
            "markdown-link-check",
            "--config",
            &temp_config.to_string_lossy(),
            &path_buf.to_string_lossy(),
        ]);

        let assert = command.current_dir(&tempdir).assert();
        let stdout = std::str::from_utf8(&assert.get_output().stdout).unwrap();
        print!("{stdout}");

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
    Command::new("prettier")
        .args(["--check", "examples/**/*.md", "internal/template/**/*.md"])
        .assert()
        .success();
}

#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn rustdoc_prettier() {
    Command::new("rustdoc-prettier")
        .args(["--check", "./**/*.rs"])
        .assert()
        .success();
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
    for entry in walkdir(true).with_file_name("Cargo.lock") {
        let entry = entry.unwrap();
        let path = entry.path();
        let manifest_path = path.with_file_name("Cargo.toml");
        Command::new("cargo")
            .args([
                "update",
                "--locked",
                "--manifest-path",
                &manifest_path.to_string_lossy(),
                "--workspace",
            ])
            .assert()
            .success();
    }
}

#[test]
fn lint() {
    let mut restriction_libs = Vec::new();
    for entry in read_dir("examples/restriction").unwrap() {
        let entry = entry.unwrap();
        if entry.path().is_dir() {
            let lib_name = entry.file_name().into_string().unwrap();
            // Exclude overscoped_allow as in the script
            if lib_name != "overscoped_allow" && lib_name != ".cargo" {
                restriction_libs.push(format!("--lib {lib_name}"));
            }
        }
    }
    let restrictions_as_flags = restriction_libs.join(" ");

    let base_flags =
        format!("--lib general --lib supplementary {restrictions_as_flags} --lib clippy");

    let mut dirs_to_lint: Vec<PathBuf> = [
        ".",
        "driver",
        "utils/linting",
        "examples/general",
        "examples/supplementary",
        "examples/restriction",
        "examples/testing/clippy",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect::<Vec<_>>();

    for entry in read_dir("examples/experimental").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path(); // path is already a PathBuf
        if path.is_dir() && entry.file_name() != ".cargo" {
            dirs_to_lint.push(path);
        }
    }

    for dir_path in &dirs_to_lint {
        eprintln!("Linting in directory: {dir_path:?}");

        let mut cmd = cargo_bin_cmd!("cargo-dylint");
        cmd.env(env::DYLINT_RUSTFLAGS, "-D warnings");
        cmd.arg("dylint");
        cmd.args(base_flags.split_whitespace());
        cmd.args(["--", "--all-features", "--tests", "--workspace"]);
        cmd.current_dir(dir_path);

        cmd.assert()
            .try_success()
            .unwrap_or_else(|error| panic!("Linting failed in {dir_path:?}: {error}"));
    }
}

#[test]
fn usage() {
    cargo_bin_cmd!("cargo-dylint")
        .args(["dylint", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage: cargo dylint"));
}

#[test]
fn version() {
    cargo_bin_cmd!("cargo-dylint")
        .args(["dylint", "--version"])
        .assert()
        .success()
        .stdout(format!("cargo-dylint {}\n", env!("CARGO_PKG_VERSION")));
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
            let filename = entry.file_name();
            filename != "target" && (include_examples || filename != "examples")
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
            entry
                .as_ref()
                .map_or(true, |entry| entry.file_name() == file_name.as_ref())
        })
    }
}
