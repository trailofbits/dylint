use dylint_internal::{
    clippy_utils::set_toolchain_channel, find_and_replace, rustup::SanitizeEnvironment,
    testing::new_template,
};
use tempfile::tempdir;
use test_log::test;

// smoelius: The channel date is one day later than the `rustc --version` date.
// smoelius: Put recent boundaries first, since they're more likely to cause problems.
const BOUNDARIES: [(&str, &str); 3] = [
    ("2023-01-19", "2023-01-20"),
    ("2022-09-08", "2022-09-09"),
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
