fn main() {
    #[cfg(docsrs)]
    add_components();
}

#[cfg(docsrs)]
fn add_components() {
    use std::{fs::read_to_string, process::Command};
    use toml::{Table, Value};

    let rust_toolchain = read_to_string("rust-toolchain").unwrap();
    let table = rust_toolchain.parse::<Table>().unwrap();
    let components = table
        .get("toolchain")
        .and_then(Value::as_table)
        .and_then(|table| table.get("components"))
        .and_then(Value::as_array)
        .unwrap();

    for component in components {
        assert!(Command::new("rustup")
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
