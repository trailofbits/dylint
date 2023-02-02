const COMPONENTS: &[&str] = &["llvm-tools-preview", "rustc-dev"];

fn main() {
    check_components();

    #[cfg(docsrs)]
    add_components();
}

fn check_components() {
    use std::{fs::read_to_string, path::Path};
    use toml::{Table, Value};

    let rust_toolchain = Path::new("rust-toolchain");

    if !rust_toolchain.try_exists().unwrap() {
        return;
    }

    let contents = read_to_string(rust_toolchain).unwrap();
    let table = contents.parse::<Table>().unwrap();
    let values = table
        .get("toolchain")
        .and_then(Value::as_table)
        .and_then(|table| table.get("components"))
        .and_then(Value::as_array)
        .unwrap();
    let components = values
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()
        .unwrap();

    assert_eq!(COMPONENTS, components);
}

#[cfg(docsrs)]
fn add_components() {
    for component in COMPONENTS {
        assert!(std::process::Command::new("rustup")
            .args([
                "component",
                "add",
                component.as_str().unwrap(),
                "--toolchain",
                "nightly"
            ])
            .status()
            .unwrap()
            .success());
    }
}
