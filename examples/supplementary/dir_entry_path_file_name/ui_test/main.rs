use std::{fs, path::PathBuf};
use walkdir::WalkDir;

fn main() {
    test_std_fs_direct_chain();
    test_walkdir_direct_chain();
    test_variable_flow();
    test_nested_in_expression();
    test_walkdir_nested();
    test_ok_direct_file_name();
    test_ok_different_type();
    test_ok_path_used_for_other();
}

// Should warn - direct chain with std::fs::DirEntry
fn test_std_fs_direct_chain() {
    for entry in fs::read_dir(".").unwrap() {
        let entry = entry.unwrap();
        let _ = entry.path().file_name();
    }
}

// Should warn - direct chain with walkdir::DirEntry
fn test_walkdir_direct_chain() {
    for entry in WalkDir::new(".") {
        let entry = entry.unwrap();
        let _ = entry.path().file_name();
    }
}

// Should warn - variable flow with std::fs::DirEntry
fn test_variable_flow() {
    for entry in fs::read_dir(".").unwrap() {
        let entry = entry.unwrap();
        let _p = entry.path();
        let f = _p.file_name();
        println!("{:?}", f);
    }
}

// Should warn - nested in expression with std::fs
fn test_nested_in_expression() {
    for entry in fs::read_dir(".").unwrap() {
        let entry = entry.unwrap();
        if entry.path().file_name() == Some("foo.txt".as_ref()) {
            println!("found");
        }
    }
}

// Should warn - walkdir in expression (but needs manual fix due to type difference)
fn test_walkdir_nested() {
    for entry in WalkDir::new(".") {
        let entry = entry.unwrap();
        let _ = entry.path().file_name();
    }
}

// Should NOT warn - already using file_name() directly
fn test_ok_direct_file_name() {
    for entry in fs::read_dir(".").unwrap() {
        let entry = entry.unwrap();
        let _ = entry.file_name();
    }

    for entry in WalkDir::new(".") {
        let entry = entry.unwrap();
        let _ = entry.file_name();
    }
}

// Should NOT warn - different type (PathBuf)
fn test_ok_different_type() {
    let path = PathBuf::from(".");
    let _ = path.as_path().file_name();
}

// Should NOT warn - path() used for something else
fn test_ok_path_used_for_other() {
    for entry in fs::read_dir(".").unwrap() {
        let entry = entry.unwrap();
        let _ = entry.path().parent();
        let _ = entry.path().exists();
    }

    for entry in WalkDir::new(".") {
        let entry = entry.unwrap();
        let _ = entry.path().parent();
    }
}
