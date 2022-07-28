use dylint_internal::{cargo::cargo_home, env};
use std::{fs::OpenOptions, io::Write, path::Path};

#[allow(unknown_lints)]
#[allow(env_cargo_path)]
fn main() {
    let cargo_home = cargo_home().unwrap();
    let out_dir = env::var(env::OUT_DIR).unwrap();

    let dylint_manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let dylint_driver_manifest_dir = if dylint_manifest_dir.starts_with(cargo_home) {
        "None".to_owned()
    } else {
        let path = dylint_manifest_dir.join("..").join("driver");

        // smoelius: Ensure the path exists at build time.
        assert!(path.is_dir());

        format!(r#"Some("{}")"#, path.to_string_lossy())
    };

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(Path::new(&out_dir).join("dylint_driver_manifest_dir.rs"))
        .unwrap();
    writeln!(
        file,
        "const DYLINT_DRIVER_MANIFEST_DIR: Option<&str> = {};",
        dylint_driver_manifest_dir
    )
    .unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
