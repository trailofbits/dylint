use std::process::{Command, Stdio};

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
    Command::new("rustc")
        .args(["-Z", "help"])
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
}
