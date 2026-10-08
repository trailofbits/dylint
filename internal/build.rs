use std::{
    env,
    ffi::OsString,
    process::{Command, Stdio},
};

fn main() {
    if is_nightly() {
        println!("cargo:rustc-cfg=nightly");
    }

    // smoelius: This fix exists in `git2`'s master branch, but we are using version 0.18. See:
    // https://github.com/rust-lang/git2-rs/pull/1143
    #[cfg(all(feature = "git", target_os = "windows"))]
    println!("cargo:rustc-link-lib=advapi32");
}

fn is_nightly() -> bool {
    Command::new(env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc")))
        .args(["-Z", "help"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_cmd::assert::OutputAssertExt;
    use std::env::{current_exe, var_os};

    #[test]
    fn is_nightly_is_quiet() {
        const CHILD: &str = "DYLINT_IS_NIGHTLY_CHILD";

        if var_os(CHILD).is_some() {
            // smoelius: The child should be launched on nightly only (see below).
            assert!(is_nightly());
            return;
        }

        // smoelius: Run the test on nightly only.
        if !is_nightly() {
            return;
        }

        // Capture inherited compiler output by running this test in a child process.
        let current_exe = current_exe().unwrap();
        let assert = Command::new(current_exe)
            .args(["tests::is_nightly_is_quiet", "--exact", "--nocapture"])
            .env(CHILD, "1")
            .assert()
            .success();

        let stdout = str::from_utf8(&assert.get_output().stdout).unwrap();
        assert!(stdout.contains("1 passed"), "{stdout}");
        assert!(!stdout.contains("Available unstable options:"), "{stdout}");
    }
}
