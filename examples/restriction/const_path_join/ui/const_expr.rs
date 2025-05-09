// run-rustfix

fn main() {
    // Test env! expressions
    let _ = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join("lib.rs");
    
    // Test concat! expressions
    let _ = std::path::PathBuf::from(concat!("path", "/to/file")).join("filename.txt");
    
    // Test nested expressions
    let dir = env!("CARGO_MANIFEST_DIR");
    let _ = std::path::PathBuf::from(dir).join("tests").join("fixtures");
    
    // Test with camino paths
    let _ = camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
} 