use dylint_internal::{
    clippy_utils::set_toolchain_channel, find_and_replace, rustup::SanitizeEnvironment,
    testing::new_template,
};
use tempfile::tempdir;
use test_log::test;

// smoelius: The channel date is one day later than the `rustc --version` date.
// smoelius: Put recent boundaries first, since they're more likely to cause problems.
// smoelius: The relevant PRs and merge commits appear before each boundary.
const BOUNDARIES: [(&str, &str); 4] = [
    // https://github.com/rust-lang/rust/pull/111633
    // https://github.com/rust-lang/rust/commit/08efb9d652c840715d15954592426e2befe13b36
    ("2023-05-18", "2023-05-19"),
    // https://github.com/rust-lang/rust/pull/106810
    // https://github.com/rust-lang/rust/commit/65d2f2a5f9c323c88d1068e8e90d0b47a20d491c
    ("2023-01-19", "2023-01-20"),
    // https://github.com/rust-lang/rust/pull/101501
    // https://github.com/rust-lang/rust/commit/87788097b776f8e3662f76627944230684b671bd
    ("2022-09-08", "2022-09-09"),
    // https://github.com/rust-lang/rust/pull/98975
    // https://github.com/rust-lang/rust/commit/0ed9c64c3e63acac9bd77abce62501696c390450
    ("2022-07-14", "2022-07-15"),
];

#[test]
fn boundary_toolchains() {
    for (before, after) in BOUNDARIES {
        for date in [before, after] {
            let channel = format!("nightly-{date}");

            let tempdir = tempdir().unwrap();

            new_template(tempdir.path()).unwrap();

            find_and_replace(
                &tempdir.path().join("Cargo.toml"),
                &[r#"s/\r?\nclippy_utils = [^\r\n]*//"#],
            )
            .unwrap();

            set_toolchain_channel(tempdir.path(), &channel).unwrap();

            dylint_internal::cargo::test(&format!("with channel `{channel}`"), false)
                .sanitize_environment()
                .current_dir(&tempdir)
                .success()
                .unwrap();
        }
    }
}
