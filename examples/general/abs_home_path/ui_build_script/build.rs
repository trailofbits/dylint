use std::path::Path;

fn main() {
    // This should NOT trigger the abs_home_path lint because we're in a build script
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    println!(
        "cargo:warning=Using manifest dir: {}",
        manifest_dir.display()
    );

    // Output rerun-if-changed directive
    println!("cargo:rerun-if-changed=build.rs");
}
