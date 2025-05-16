#[cfg(not(feature = "packaging"))]
fn main() {}

#[cfg(feature = "packaging")]
fn main() {
    use std::{
        env::var,
        ffi::OsStr,
        fs::File,
        path::{Path, PathBuf},
    };
    use tar::Builder;
    use walkdir::WalkDir;

    #[cfg_attr(dylint_lib = "env_literal", allow(env_literal))]
    let outdir = var("OUT_DIR").map(PathBuf::from).unwrap();
    let path_buf = outdir.join("template.tar");
    let file = File::create(path_buf).unwrap();
    let mut archive = Builder::new(file);
    let root = Path::new("template");
    for result in WalkDir::new(root).into_iter().filter_entry(|entry| {
        let filename = entry.file_name();
        filename != OsStr::new("Cargo.lock") && filename != OsStr::new("target")
    }) {
        let entry = result.unwrap();
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let mut file = File::open(path).unwrap();
        let path_stripped = path.strip_prefix(root).unwrap();
        archive.append_file(path_stripped, &mut file).unwrap();
    }
}
