//! Extracts the `clippy_utils` revision from the project's Cargo.toml file and writes it to
//! `out_dir/clippy_utils_rev.txt`.

use dylint_internal::env;
use std::{
    fs::{read_to_string, write},
    path::PathBuf,
};

fn main() {
    let out_dir = env::var(env::OUT_DIR).unwrap();
    let contents = read_to_string("Cargo.toml").unwrap();
    let document = contents.parse::<toml::Table>().unwrap();
    let rev = document
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("clippy_utils"))
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("rev"))
        .and_then(toml::Value::as_str)
        .unwrap();
    let path_buf = PathBuf::from(out_dir).join("clippy_utils_rev.txt");
    write(path_buf, rev).unwrap();
}
