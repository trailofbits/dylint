#![cfg(not(coverage))]

use dylint_internal::{
    clippy_utils::set_toolchain_channel, env, find_and_replace, rustup::SanitizeEnvironment,
    testing::new_template, CommandExt,
};
use tempfile::tempdir;

// smoelius: The channel date is one day later than the `rustc --version` date.
// smoelius: Put recent boundaries first, since they're more likely to cause problems.
// smoelius: The relevant PRs and merge commits appear before each boundary.
const BOUNDARIES: &[(&str, &str)] = &[
    // https://github.com/rust-lang/rust/pull/121780
    // https://github.com/rust-lang/rust/commit/1547c076bfec8abb819d6a81e1e4095d267bd5b4
    // https://github.com/rust-lang/rust/pull/121969
    // https://github.com/rust-lang/rust/commit/13b971209a27127a0446e015edb033f903da44e4
    // smoelius: Note that 2024-03-03 through 2024-03-06 are skipped because of the following
    // issue: https://github.com/rust-lang/rust/issues/121889
    ("2024-03-02", "2024-03-07"),
    // https://github.com/rust-lang/rust/pull/119146
    // https://github.com/rust-lang/rust/commit/2271c26e4a8e062bb00d709d0ccb5846e0c341b9
    ("2023-12-26", "2023-12-27"),
    // https://github.com/rust-lang/rust/pull/119171
    // https://github.com/rust-lang/rust/commit/e0d7a72c46d554cb63a1f91a523bfc9e6e37d886
    ("2023-12-23", "2023-12-24"),
    // https://github.com/rust-lang/rust/pull/119063
    // https://github.com/rust-lang/rust/commit/cda4736f1eaad8af6f49388baa9b7e480df8e329
    ("2023-12-18", "2023-12-19"),
    // smoelius: `cargo-platform v0.1.8` requires rustc 1.73 or newer.
    // https://github.com/rust-lang/rust/pull/112692
    // https://github.com/rust-lang/rust/commit/b6144cd843d6eb6acc086797ea37e0c69c892b90
    // ("2023-06-28", "2023-06-29"),
    // https://github.com/rust-lang/rust/pull/111748
    // https://github.com/rust-lang/rust/commit/70e04bd88d85cab8ed110ace5a278fab106d0ef5
    // ("2023-05-29", "2023-05-30"),

    // smoelius: `cargo-util v0.2.7` requires rustc 1.72.0 or newer.
    // https://github.com/rust-lang/rust/pull/111633
    // https://github.com/rust-lang/rust/commit/08efb9d652c840715d15954592426e2befe13b36
    // ("2023-05-18", "2023-05-19"),

    // smoelius: `home v0.5.9` requires rustc 1.70.0 or newer
    // https://github.com/rust-lang/rust/pull/106810
    // https://github.com/rust-lang/rust/commit/65d2f2a5f9c323c88d1068e8e90d0b47a20d491c
    // ("2023-01-19", "2023-01-20"),

    // smoelius: `toml v0.7.8` requires rustc 1.66.0 or newer.
    // https://github.com/rust-lang/rust/pull/101501
    // https://github.com/rust-lang/rust/commit/87788097b776f8e3662f76627944230684b671bd
    // ("2022-09-08", "2022-09-09"),

    // smoelius: `git2-0.17.2` requires `std::ffi::c_uint`, which was introduced in Rust 1.64.0
    // (2022-09-22): https://doc.rust-lang.org/stable/std/ffi/type.c_uint.html

    // https://github.com/rust-lang/rust/pull/98975
    // https://github.com/rust-lang/rust/commit/0ed9c64c3e63acac9bd77abce62501696c390450
    // ("2022-07-14", "2022-07-15"),
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
                "\r?\nclippy_utils = [^\r\n]*",
                "",
            )
            .unwrap();

            set_toolchain_channel(tempdir.path(), &channel).unwrap();

            dylint_internal::cargo::test(&format!("with channel `{channel}`"))
                .build()
                .sanitize_environment()
                .current_dir(&tempdir)
                .success()
                .unwrap_or_else(|_| panic!("failed with channel `{channel}`"));

            if std::env::var(env::CI).is_ok() {
                assert!(std::process::Command::new("rustup")
                    .args(["uninstall", &channel])
                    .status()
                    .unwrap()
                    .success());
            }
        }
    }
}
