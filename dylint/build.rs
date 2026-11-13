use dylint_internal::{cargo::cargo_home, env};
use std::{
    fs::{OpenOptions, read, read_dir},
    hash::{DefaultHasher, Hash, Hasher},
    io::Write,
    path::Path,
};

fn main() {
    write_dylint_driver_manifest_dir();

    #[cfg(feature = "__cargo_cli")]
    println!("cargo:rustc-cfg=__library_packages");

    println!("cargo:rerun-if-changed=build.rs");
}

fn write_dylint_driver_manifest_dir() {
    let cargo_home = cargo_home().unwrap();
    let out_dir = env::var(env::OUT_DIR).unwrap();

    #[cfg_attr(dylint_lib = "general", allow(abs_home_path))]
    let dylint_manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let local_driver = !(cfg!(feature = "__driver_from_crates_io")
        || dylint_manifest_dir.starts_with(cargo_home)
        || dylint_manifest_dir
            .parent()
            .is_some_and(|path| path.ends_with("target/package"))
        || env::var(env::DOCS_RS).is_ok());
    let dylint_driver_manifest_dir = if local_driver {
        let path_buf = dylint_manifest_dir.join("../driver");

        // smoelius: Ensure the path exists at build time.
        assert!(
            path_buf.is_dir(),
            "`{}` is not a directory",
            path_buf.display()
        );

        format!("Some({:?})", path_buf.to_string_lossy())
    } else {
        "None".to_owned()
    };

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(Path::new(&out_dir).join("dylint_driver_manifest_dir.rs"))
        .unwrap();
    writeln!(
        file,
        r#"#[cfg_attr(dylint_lib = "general", allow(abs_home_path))]
    const DYLINT_DRIVER_MANIFEST_DIR: Option<&str> = {dylint_driver_manifest_dir};"#
    )
    .unwrap();
    let version = env!("CARGO_PKG_VERSION");
    let key = if local_driver {
        let mut hasher = DefaultHasher::new();
        let root = dylint_manifest_dir.parent().unwrap();
        root.canonicalize().unwrap().hash(&mut hasher);
        for path in [
            "Cargo.toml",
            "Cargo.lock",
            "dylint/build.rs",
            "dylint/src/driver_builder",
            "driver/Cargo.toml",
            "driver/Cargo.lock",
            "driver/src",
            "driver/build",
            "internal/Cargo.toml",
            "internal/build.rs",
            "internal/src",
        ] {
            hash_driver_source(&root.join(path), &mut hasher);
        }
        format!("{version}-dev-{:016x}", hasher.finish())
    } else {
        version.to_owned()
    };
    writeln!(file, "const DYLINT_DRIVER_CACHE_KEY: &str = {key:?};").unwrap();
}

fn hash_driver_source(path: &Path, hasher: &mut DefaultHasher) {
    println!("cargo:rerun-if-changed={}", path.display());
    path.hash(hasher);
    if path.is_dir() {
        let mut paths = read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            hash_driver_source(&path, hasher);
        }
    } else {
        read(path).unwrap().hash(hasher);
    }
}
