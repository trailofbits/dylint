use super::*;
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
            &[] as &[&Path]
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
            &[] as &[&Path]
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
            &[] as &[&Path]
        )
        .unwrap()
    );
}
