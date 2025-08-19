use assert_cmd::Command;
use dylint_internal::env;
use similar_asserts::SimpleDiff;
use std::{
    fs::{read_to_string, write},
    path::PathBuf,
    process,
    str::FromStr,
};

const TARGETS: [&str; 3] = [
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
];

#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn duplicate_dependencies() {
    for target in TARGETS {
        let mut command = Command::new("cargo");
        command.current_dir("..");
        command.args(["tree", "--duplicates", "--edges=normal", "--target", target]);
        let assert = command.assert().success();

        let stdout_actual = std::str::from_utf8(&assert.get_output().stdout).unwrap();
        let package_versions = stdout_actual
            .lines()
            .filter(|line| line.chars().next().is_some_and(char::is_alphabetic))
            .map(|line| {
                <[_; 2]>::try_from(line.split_ascii_whitespace().take(2).collect::<Vec<_>>())
                    .unwrap()
            })
            .collect::<Vec<_>>();
        #[allow(clippy::format_collect)]
        let stdout_filtered = {
            const PACKAGE: usize = 0;
            const VERSION: usize = 1;
            let mut package_versions_filtered = package_versions
                .windows(2)
                .filter(|w| w[0][PACKAGE] == w[1][PACKAGE])
                .filter(|w| w[0][VERSION] != w[1][VERSION])
                .flatten()
                .collect::<Vec<_>>();
            // smoelius: If `package_versions` contains three versions of a package, then
            // `package_versions_filtered` will contain:
            // ```
            // package version-0
            // package version-1
            // package version-1
            // package version-2
            // ```
            package_versions_filtered.dedup();
            package_versions_filtered
                .into_iter()
                .map(|package_version| {
                    format!(
                        "{} {}\n",
                        package_version[PACKAGE], package_version[VERSION]
                    )
                })
                .collect::<String>()
        };

        let path = PathBuf::from(format!("duplicate_dependencies/{target}.txt"));

        let stdout_expected = read_to_string(&path).unwrap();

        if env::enabled("BLESS") {
            write(path, stdout_filtered).unwrap();
        } else {
            assert!(
                stdout_expected == stdout_filtered,
                "{}",
                SimpleDiff::from_str(&stdout_expected, &stdout_filtered, "left", "right")
            );
        }
    }
}

// smoelius: `supply_chain` is the only test that uses `supply_chain.json`. So there is no race.
#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn supply_chain() {
    let mut command = process::Command::new("cargo");
    command.args(["supply-chain", "update", "--cache-max-age=0s"]);
    let _: process::ExitStatus = command.status().unwrap();

    for target in TARGETS {
        let mut command = Command::new("cargo");
        command.current_dir("..");
        command.args(["supply-chain", "json", "--no-dev", "--target", target]);
        let assert = command.assert().success();

        let stdout_actual = std::str::from_utf8(&assert.get_output().stdout).unwrap();
        // smoelius: Sanity. (I have nothing against Redox OS.)
        assert!(!stdout_actual.contains("redox"));
        let mut value = serde_json::Value::from_str(stdout_actual).unwrap();
        remove_avatars(&mut value);
        let stdout_normalized = serde_json::to_string_pretty(&value).unwrap();

        let path = PathBuf::from(format!("supply_chain/{target}.json"));

        let stdout_expected = read_to_string(&path).unwrap();

        if env::enabled("BLESS") {
            write(path, stdout_normalized).unwrap();
        } else {
            assert!(
                stdout_expected == stdout_normalized,
                "{}",
                SimpleDiff::from_str(&stdout_expected, &stdout_normalized, "left", "right")
            );
        }
    }
}

fn remove_avatars(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
        serde_json::Value::Array(array) => {
            for value in array {
                remove_avatars(value);
            }
        }
        serde_json::Value::Object(object) => {
            object.retain(|key, value| {
                if key == "avatar" {
                    return false;
                }
                remove_avatars(value);
                true
            });
        }
    }
}

#[test]
fn unmaintained() {
    Command::new("cargo")
        .current_dir("..")
        .args(["unmaintained", "--color=never", "--fail-fast"])
        .assert()
        .success();
}
