use std::{
    ffi::OsStr,
    fs::{read_dir, File},
    io::{BufRead, BufReader},
    path::Path,
};
use test_log::test;

#[test]
fn all_tests_use_test_log() {
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");

    for entry in read_dir(tests).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension() != Some(OsStr::new("rs")) {
            continue;
        }
        let file = File::open(&path).unwrap();
        assert!(
            BufReader::new(file)
                .lines()
                .any(|line| { line.unwrap().trim_start() == "use test_log::test;" }),
            "{path:?} does not use `test_log::test`"
        );
    }
}
