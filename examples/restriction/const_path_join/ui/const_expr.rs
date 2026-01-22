// run-rustfix

fn main() {
    // Test env! expressions
    let _ = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("lib.rs");

    // Test nested expressions
    let dir = env!("CARGO_MANIFEST_DIR");
    let _ = std::path::PathBuf::from(dir).join("tests").join("fixtures");

    // Test with camino paths
    let _ = camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
}

#[allow(dead_code)]
mod negative_tests {
    const CATEGORY: &str = "restriction";
    const LIB_NAME: &str = "const_path_join";

    fn foo() {
        let _ = std::path::Path::new("examples")
            .join(CATEGORY)
            .join(LIB_NAME);
    }
}
