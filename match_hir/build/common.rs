use std::{
    fs::{File, OpenOptions},
    path::Path,
};

pub fn create(out_dir: &str, file_name: &str) -> std::io::Result<File> {
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(Path::new(out_dir).join(file_name))
}
