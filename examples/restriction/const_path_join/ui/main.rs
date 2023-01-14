// run-rustfix

fn main() {
    let _ = std::path::Path::new("..").join("target");
    let _ = std::path::PathBuf::from("..").join("target");
    let _ = std::path::PathBuf::from("..").join("target").as_path();
    let _ = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target");

    let _ = camino::Utf8Path::new("..").join("target");
    let _ = camino::Utf8PathBuf::from("..").join("target");
    let _ = camino::Utf8PathBuf::from("..").join("target").as_path();
    let _ = camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target");
}
