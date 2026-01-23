use dylint_internal::{cargo::cargo_home, env};
use std::{fs::OpenOptions, io::Write, path::Path};

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

    let dylint_driver_manifest_dir = if dylint_manifest_dir.starts_with(cargo_home)
        || dylint_manifest_dir
            .parent()
            .is_some_and(|path| path.ends_with("target/package"))
        || env::var(env::DOCS_RS).is_ok()
    {
        "None".to_owned()
    } else {
        let path_buf = dylint_manifest_dir.join("../driver");

        // smoelius: Ensure the path exists at build time.
        assert!(
            path_buf.is_dir(),
            "`{}` is not a directory",
            path_buf.display()
        );

        format!(
            r#"Some("{}")"#,
            path_buf.to_string_lossy().replace('\\', "\\\\")
        )
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
}
