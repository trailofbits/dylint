use anyhow::{Context, Result, anyhow};
use dylint_internal::{
    CommandExt, clone, env::enabled, rustup::SanitizeEnvironment, testing::cargo_dylint,
};
use regex::Regex;
use semver::Version;
use snapbox::{Assert, Data, assert::DEFAULT_ACTION_ENV};
use std::{
    fs::read_to_string,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    sync::LazyLock,
};
use tempfile::tempdir;

const SHORT_ID_LEN: usize = 7;

const DYLINT_URL: &str = "https://github.com/trailofbits/dylint";

// smoelius: Each of the following commits is just before an "Upgrade examples" commit. In the
// upgrades, the changes to the `restriction` lints tend to be small. Thus, the auto-correct
// option should be able to generate fixes for many of them.
// smoelius: Each tuple's third component is whether compilation should ultimately succeed.
const REVS_AND_RUST_VERSIONS: &[(&str, &str, bool)] = &[
    // smoelius: "Upgrade examples" commit:
    // https://github.com/trailofbits/dylint/commit/53e617e844b1f2c0d953d67b47a525381ec094c7
    ("5343770f654d2bcd0fe246bb333e1a2b63048df0", "1.92.0", false),
    // smoelius: "Upgrade examples" commit:
    // https://github.com/trailofbits/dylint/commit/5d5717e3e314c8bdb1f6d0bcd3852d1059a2b482
    ("1e03ecf75981d94a7917866454b7bd0214916165", "1.91.0", true),
    // smoelius: "Upgrade examples" commit:
    // https://github.com/trailofbits/dylint/commit/c45f27a8068ff9de1efa88f7fde1574dd04ed8c2
    ("80ea87d485a80c61efc65ab49dd7b28b54554969", "1.90.0", false),
    // smoelius: "Upgrade examples" commit:
    // https://github.com/trailofbits/dylint/commit/24407a3d328ad0d2f6318ba186b2ac126713622f
    ("a93c166d88662b6bfc29ca7c31177a6c6f2b897b", "1.89.0", false),
    // smoelius: "Upgrade examples" commit:
    // https://github.com/trailofbits/dylint/commit/33969746aef6947c68d7adb55137ce8a13d9cc47
    ("5b3792515ac255fdb06a31b10eb3c9f7949a3ed5", "1.80.0", true),
    // smoelius: "Upgrade examples" commit:
    // https://github.com/trailofbits/dylint/commit/7bc453f0778dee3b13bc1063773774304ac96cad
    ("23c08c8a0b043d26f66653bf173a0e6722a2d699", "1.79.0", true),
];

#[test]
fn upgrade_restriction_examples_with_auto_correct() {
    for &(rev, rust_version, should_succeed) in REVS_AND_RUST_VERSIONS {
        let short_id = &rev[..SHORT_ID_LEN];
        let stderr_path = Path::new("tests/auto_correct")
            .join(short_id)
            .with_extension("stderr");
        let expected_stderr = read_to_string(stderr_path).unwrap();

        let tempdir = tempdir().unwrap();

        clone(DYLINT_URL, rev, tempdir.path(), true).unwrap();

        let cargo_dylint = cargo_dylint("..").unwrap();
        let mut command = Command::new(cargo_dylint);
        command.args([
            "dylint",
            "upgrade",
            &tempdir
                .path()
                .join("examples/restriction")
                .to_string_lossy(),
            "--auto-correct",
            "--rust-version",
            rust_version,
        ]);
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::piped());

        let output = command.logged_output(false).unwrap();

        assert_expected_is_superset_of_actual(
            &expected_stderr,
            str::from_utf8(&output.stderr).unwrap(),
        );

        if enabled("DEBUG_DIFF") {
            let mut command = Command::new("git");
            command.current_dir(&tempdir);
            command.args(["--no-pager", "diff", "--", "*.rs"]);
            command.success().unwrap();
        }

        let status = dylint_internal::cargo::check("auto-corrected, upgraded library package")
            .build()
            .sanitize_environment()
            .current_dir(tempdir.path().join("examples/restriction"))
            .env("RUSTFLAGS", "--allow=warnings")
            .arg("--quiet")
            .status()
            .unwrap();

        assert_eq!(should_succeed, status.success());
    }
}

static FOUND_N_HIGHLIGHTS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Found [0-9]+ highlights in [0-9]+ seconds$").unwrap());

#[allow(clippy::format_collect)]
fn assert_expected_is_superset_of_actual(expected: &str, actual: &str) {
    let mut expected_iter = expected.lines().peekable();
    let mut expected_index = 1;
    let mut actual_iter = actual.lines().peekable();
    let mut actual_index = 1;
    let mut last_expected_index_and_line = None;
    loop {
        if let Some(&expected_line) = expected_iter.peek() {
            if let Some(&actual_line) = actual_iter.peek()
                && Assert::new()
                    .action_env(DEFAULT_ACTION_ENV)
                    .try_eq(
                        None,
                        Data::from(actual_line.replace("\\\\", "/")),
                        Data::from(expected_line),
                    )
                    .is_ok()
            {
                let _ = expected_iter.next();
                expected_index += 1;
                let _ = actual_iter.next();
                actual_index += 1;
                last_expected_index_and_line = None;
                continue;
            }
            // smoelius: The expected line and actual lines do not match, but there _is_ an
            // expected line. Record it for diagnostic purposes.
            if last_expected_index_and_line.is_none() {
                last_expected_index_and_line = Some((expected_index, expected_line));
            }
        }
        // smoelius: On Linux, macOS, and Windows, I see "Updating files: <percentage>%" in the
        // logs. Googling suggests that git generates these messages when it checks out the HEAD
        // branch. I cannot figure out how to prevent git from generating these messages. So if
        // the actual line starts with "Updating files: ", throw it away.
        if actual_iter
            .peek()
            .is_some_and(|actual_line| actual_line.starts_with("Updating files: "))
        {
            let _ = actual_iter.next();
            actual_index += 1;
            continue;
        }
        // smoelius: Check for actual lines that require the comparison to be aborted. This
        // check must be performed before the check of whether expected lines remain. Otherwise
        // a "Found N highlights" line could be carried to the end and produce a failed match.
        if let Some(actual_line) = actual_iter.peek()
            && (actual_line.starts_with("Warning: Found diagnostic error with no spans: ")
                || FOUND_N_HIGHLIGHTS_RE.is_match(actual_line))
        {
            #[allow(clippy::explicit_write)]
            writeln!(
                std::io::stderr(),
                "unexpected number of highlights at actual line {actual_index}; aborting comparison: {actual_line}"
            )
            .unwrap();
            return;
        }
        // smoelius: Actual only has to be a subset of expected. So expected can contain lines
        // not in actual.
        if expected_iter.peek().is_some() {
            let _ = expected_iter.next();
            expected_index += 1;
            continue;
        }
        // smoelius: If there are no more actual lines, break.
        let actual_line = match actual_iter.next() {
            Some(actual_line) => actual_line.replace("\\\\", "/"),
            None => break,
        };
        let (last_expected_index, last_expected_line) =
            last_expected_index_and_line.unwrap_or((0, "??"));
        panic!(
            "\
mismatch between expected line at {last_expected_index}: {last_expected_line}
                   actual line at {actual_index}: {actual_line}
expected stderr:\n```\n{}```
  actual stderr:\n```\n{}```",
            expected
                .lines()
                .enumerate()
                .map(|(i, line)| format!("{:>4}: {}\n", i + 1, line))
                .collect::<String>(),
            actual
                .lines()
                .enumerate()
                .map(|(i, line)| format!("{:>4}: {}\n", i + 1, line))
                .collect::<String>()
        );
    }
}

#[allow(dead_code)]
fn rust_version(path: &Path) -> Result<Version> {
    let re = Regex::new(r#"^clippy_utils = .*\btag = "rust-([^"]*)""#).unwrap();
    let manifest = path.join("Cargo.toml");
    let contents = read_to_string(&manifest).with_context(|| {
        format!(
            "`read_to_string` failed for `{}`",
            manifest.to_string_lossy()
        )
    })?;
    let rust_version = contents
        .lines()
        .find_map(|line| re.captures(line).map(|captures| captures[1].to_owned()))
        .ok_or_else(|| anyhow!("Could not determine `clippy_utils` version"))?;
    Version::parse(&rust_version).map_err(Into::into)
}
