#![warn(const_path_join)]

use std::path::PathBuf;

fn main() {
    // Test cases with literal strings
    let _ = PathBuf::from("foo").join("bar");
    let _ = PathBuf::from("foo").join("bar").join("baz");
    let _ = PathBuf::new().join("foo").join("bar");

    // Test cases with constant expressions
    let _ = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let _ = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("debug");
    
    // Test cases with mixed literal strings and constant expressions
    let _ = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join("lib.rs");
    let _ = PathBuf::from("target").join(env!("CARGO_PKG_NAME")).join("debug");

    // Test cases with camino::Utf8PathBuf
    use camino::Utf8PathBuf;
    let _ = Utf8PathBuf::from("foo").join("bar");
    let _ = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
} 