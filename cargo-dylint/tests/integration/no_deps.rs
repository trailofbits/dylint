use assert_cmd::{Command, cargo::cargo_bin_cmd};
use dylint_internal::env;

#[test]
fn current_dir() {
    test(|command| {
        command.current_dir("../fixtures/no_deps/a");
    });
}

#[test]
fn manifest_path() {
    test(|command| {
        command
            .current_dir("../fixtures/no_deps")
            .args(["--manifest-path", "a/Cargo.toml"]);
    });
}

#[test]
fn package() {
    test(|command| {
        command
            .current_dir("../fixtures/no_deps")
            .args(["--package", "a"]);
    });
}

fn test(f: impl Fn(&mut Command)) {
    for no_deps in [false, true] {
        let mut command = base_command();

        f(&mut command);

        if no_deps {
            command.arg("--no-deps").assert().success();
        } else {
            command.assert().failure();
        }
    }
}

fn base_command() -> Command {
    #[cfg_attr(dylint_lib = "general", allow(abs_home_path))]
    let mut command = cargo_bin_cmd!("cargo-dylint");
    command.env(env::RUSTFLAGS, "-D warnings").args([
        "dylint",
        "--lib",
        "question_mark_in_expression",
    ]);
    command
}
