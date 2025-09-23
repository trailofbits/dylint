use std::process::{Command, Stdio};

fn main() {
    if is_nightly() {
        println!("cargo:rustc-cfg=nightly");
    }

    #[cfg(all(any(), feature = "packaging"))]
    build_template_tar();

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

#[cfg(all(any(), feature = "packaging"))]
fn build_template_tar() {
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
