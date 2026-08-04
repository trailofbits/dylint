const COMPONENTS: &[&str] = &["llvm-tools-preview", "rustc-dev"];

fn main() {
    check_components();

    #[cfg(docsrs)]
    add_components();
}

fn check_components() {
    use dylint_internal::{clippy_utils, env};
    use std::{env::var_os, path::PathBuf};
    use toml::{Table, Value};

    let manifest_dir = var_os(env::CARGO_MANIFEST_DIR).unwrap();
    let path = PathBuf::from(manifest_dir);

    let Some((_, contents)) = clippy_utils::read_rust_toolchain_file(&path).unwrap() else {
        return;
    };

    let table = contents.parse::<Table>().unwrap();
    let array = table
        .get("toolchain")
        .and_then(Value::as_table)
        .and_then(|table| table.get("components"))
        .and_then(Value::as_array)
        .unwrap();
    let components = array
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()
        .unwrap();

    assert_eq!(COMPONENTS, components);
}

#[cfg(docsrs)]
fn add_components() {
    for component in COMPONENTS {
        assert!(
            std::process::Command::new("rustup")
                .args(["component", "add", component, "--toolchain", "nightly"])
                .status()
                .unwrap()
                .success()
        );
    }
}
