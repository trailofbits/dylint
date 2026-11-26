#![cfg(not(coverage))]

use assert_cmd::Command;
use dylint_internal::env;
use std::{
    env::var,
    ffi::OsStr,
    fs::{read_to_string, write},
    io::{Write, stderr},
    path::Path,
};

#[test]
fn markdown_link_check() {
    // Skip the test if GITHUB_TOKEN is not available
    #[allow(clippy::explicit_write)]
    let Ok(token) = var(env::GITHUB_TOKEN) else {
        writeln!(
            stderr(),
            "Skipping `markdown_link_check` test as {} environment variable is not set",
            env::GITHUB_TOKEN
        )
        .unwrap();
        writeln!(
            stderr(),
            "To run this test, set the token: {}=your_token cargo test ...",
            env::GITHUB_TOKEN
        )
        .unwrap();
        return;
    };

    let tempdir = tempfile::tempdir().unwrap();

    Command::new("npm")
        .args(["install", "markdown-link-check"])
        .current_dir(&tempdir)
        .assert()
        .success();

    // smoelius: https://github.com/rust-lang/crates.io/issues/788
    let config = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/markdown_link_check.json"
    ));

    // Read the original config content
    let mut config_content = read_to_string(config).unwrap();
    let temp_config = tempdir.path().join("markdown_link_check.json");

    // Replace ${GITHUB_TOKEN} with the actual token
    config_content = config_content.replace("${GITHUB_TOKEN}", &token);
    write(&temp_config, config_content).unwrap();

    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    let walk = ignore::WalkBuilder::new(root)
        .standard_filters(false)
        .git_ignore(true)
        .build();

    for entry in walk {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension() != Some(OsStr::new("md")) {
            continue;
        }

        let mut command = Command::new("npx");
        command.args([
            "markdown-link-check",
            "--config",
            &temp_config.to_string_lossy(),
            &path.to_string_lossy(),
        ]);

        let assert = command.current_dir(&tempdir).assert();
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
