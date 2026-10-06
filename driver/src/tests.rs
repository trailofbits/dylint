use super::*;
use dylint_internal::CommandExt;
use rustc_version::{Channel, version_meta};

#[test]
fn channel_is_nightly() {
    assert!(matches!(version_meta().unwrap().channel, Channel::Nightly));
}

#[test]
fn no_rustc() {
    assert_eq!(
        vec!["rustc", "--crate-name", "name"],
        rustc_args(
            &["--crate-name", "name"],
            None,
            &[] as &[&str],
            &[] as &[&Path],
            None
        )
        .unwrap()
    );
}

#[test]
fn plain_rustc() {
    assert_eq!(
        vec!["rustc", "--crate-name", "name"],
        rustc_args(
            &["rustc", "--crate-name", "name"],
            None,
            &[] as &[&str],
            &[] as &[&Path],
            None
        )
        .unwrap()
    );
}

#[test]
fn qualified_rustc() {
    assert_eq!(
        vec!["/bin/rustc", "--crate-name", "name"],
        rustc_args(
            &["/bin/rustc", "--crate-name", "name"],
            None,
            &[] as &[&str],
            &[] as &[&Path],
            None
        )
        .unwrap()
    );
}

#[test]
fn untracked_state_is_passed_to_rustc() {
    assert_eq!(
        vec![
            "rustc",
            "-Cmetadata=dylint-untracked-state-0123456789abcdef",
            "--crate-name",
            "name"
        ],
        rustc_args(
            &["--crate-name", "name"],
            None,
            &[] as &[&str],
            &[] as &[&Path],
            Some("0123456789abcdef")
        )
        .unwrap()
    );
}

// Exercise the generated arguments with rustc, rather than merely checking their spelling.
// Cargo's metadata must accumulate with ours, and returning to a previous hash must select
// the original incremental session.
#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn untracked_state_selects_incremental_session() {
    let source = library_path("incremental", "main.rs");
    let dir = source.parent().unwrap();
    let incremental = dir.join("incremental");
    if incremental.exists() {
        std::fs::remove_dir_all(&incremental).unwrap();
    }
    std::fs::write(&source, "fn main() {}\n").unwrap();

    let compile = |metadata, state| {
        let args = rustc_args(
            &[
                "--crate-name",
                "untracked_state",
                "--emit=metadata",
                "--out-dir",
                dir.to_str().unwrap(),
                "-C",
                &format!("incremental={}", incremental.display()),
                "-C",
                &format!("metadata={metadata}"),
                source.to_str().unwrap(),
            ],
            None,
            &[] as &[&str],
            &[] as &[&Path],
            Some(state),
        )
        .unwrap();
        std::process::Command::new(&args[0])
            .args(&args[1..])
            .logged_output(true)
            .unwrap();
        std::fs::read_dir(&incremental)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<BTreeSet<_>>()
    };

    // The first compilation creates one session. Identical inputs select the same session.
    let first = compile("cargo", "0123456789abcdef");
    assert_eq!(1, first.len());
    assert_eq!(first, compile("cargo", "0123456789abcdef"));

    // Changing only Dylint's hash creates a separate session and retains the original one.
    let second = compile("cargo", "fedcba9876543210");
    assert_eq!(2, second.len());
    assert!(first.is_subset(&second));
    // Returning to the original hash selects its existing session without creating another.
    assert_eq!(second, compile("cargo", "0123456789abcdef"));

    // Cargo's metadata still affects the session: Dylint's argument must accumulate with it.
    let third = compile("cargo-other", "0123456789abcdef");
    assert_eq!(3, third.len());
    assert!(second.is_subset(&third));
}

#[test]
fn no_untracked_state_without_libraries() {
    assert_eq!(None, untracked_state::hash(&[], &[] as &[&Path]).unwrap());
}

// Rebuilding a library overwrites the same path, so the hash must depend on more than the paths
// in `DYLINT_LIBS`. The two writes have equal length, so this fails if the hash reflects only
// the libraries' sizes.
#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn untracked_state_changes_when_library_is_rebuilt() {
    let path = library_path("rebuilt", "liblibrary@toolchain.so");

    std::fs::write(&path, b"registers lint_a").unwrap();
    let before = untracked_state::hash(&[], &[&path]).unwrap();

    std::fs::write(&path, b"registers lint_b").unwrap();
    let after = untracked_state::hash(&[], &[&path]).unwrap();

    assert!(before.is_some());
    assert_ne!(before, after);
}

#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn untracked_state_is_independent_of_path_order() {
    let path_a = library_path("order", "liba@toolchain.so");
    let path_b = library_path("order", "libb@toolchain.so");
    std::fs::write(&path_a, b"a").unwrap();
    std::fs::write(&path_b, b"b").unwrap();

    assert_eq!(
        untracked_state::hash(&[], &[&path_a, &path_b]).unwrap(),
        untracked_state::hash(&[], &[&path_b, &path_a]).unwrap()
    );
}

// `CARGO_PRIMARY_PACKAGE` and `DYLINT_NO_DEPS` together determine whether `Callbacks::config`
// registers any lints at all, so they must affect the hash.
#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn untracked_state_depends_on_primary_package() {
    let path = library_path("primary_package", "liblibrary@toolchain.so");
    std::fs::write(&path, b"contents").unwrap();

    let primary =
        untracked_state::hash(&[(env::CARGO_PRIMARY_PACKAGE, Some("1"))], &[&path]).unwrap();
    let not_primary =
        untracked_state::hash(&[(env::CARGO_PRIMARY_PACKAGE, None)], &[&path]).unwrap();

    assert_ne!(primary, not_primary);
}

#[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
#[test]
fn untracked_state_depends_on_no_deps() {
    let path = library_path("no_deps", "liblibrary@toolchain.so");
    std::fs::write(&path, b"contents").unwrap();

    let with_no_deps =
        untracked_state::hash(&[(env::DYLINT_NO_DEPS, Some("1"))], &[&path]).unwrap();
    let without_no_deps =
        untracked_state::hash(&[(env::DYLINT_NO_DEPS, Some("0"))], &[&path]).unwrap();

    assert_ne!(with_no_deps, without_no_deps);
}

// `OUT_DIR` is used rather than a temporary directory so that the driver needs no additional
// dev-dependency. Each test gets its own subdirectory so that the tests do not interfere with
// one another.
fn library_path(test: &str, filename: &str) -> PathBuf {
    let dir = Path::new(concat!(env!("OUT_DIR"), "/untracked_state")).join(test);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(filename)
}
