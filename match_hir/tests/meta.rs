use assert_cmd::Command;
use snapbox::assert_data_eq;
use std::{fs::read_to_string, path::Path};

#[test]
fn self_reflective_match_exact() {
    self_reflective_match(false);
}

#[test]
fn self_reflective_match_wildcard() {
    self_reflective_match(true);
}

fn self_reflective_match(wildcard: bool) {
    Command::new("cargo")
        .arg("build")
        .current_dir("reflective_match")
        .assert()
        .success();

    let mut command = Command::new("cargo");
    command.args(["dylint", "--path", "reflective_match", "--", "--quiet"]);
    if wildcard {
        command.env("WILDCARD", "1");
    }
    let assert = command.assert().success();
    let stderr_expected = read_to_string(Path::new("tests").join(format!(
        "reflective_match_{}.stderr",
        if wildcard { "wildcard" } else { "exact" }
    )))
    .unwrap();
    let stderr_actual = std::str::from_utf8(&assert.get_output().stderr).unwrap();
    assert_data_eq!(stderr_actual, &stderr_expected);
}
