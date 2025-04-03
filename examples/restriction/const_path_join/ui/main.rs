// run-rustfix

fn main() {
    let _ = std::path::Path::new("..").join("target");
    let _ = std::path::PathBuf::from("..").join("target");
    let _ = std::path::PathBuf::from("..").join("target").as_path();
    let _ = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target");
        
    // Added test cases for different environment variables
    let _ = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/config.json");
    let _ = std::path::Path::new(env!("CARGO_PKG_NAME")).join("data.txt");
    let _ = std::path::PathBuf::from(env!("CARGO_PKG_VERSION")).join("logs");
    
    // Added test cases from error messages
    let _ = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../dylint-link");
    let _ = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");

    let _ = camino::Utf8Path::new("..").join("target");
    let _ = camino::Utf8PathBuf::from("..").join("target");
    let _ = camino::Utf8PathBuf::from("..").join("target").as_path();
    let _ = camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target");
        
    // Added test cases for camino paths with different environment variables
    let _ = camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/icons");
    let _ = camino::Utf8PathBuf::from(env!("OUT_DIR")).join("generated");
}
