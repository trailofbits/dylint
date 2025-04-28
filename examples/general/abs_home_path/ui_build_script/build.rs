use std::path::Path;

fn main() {
    // This should NOT trigger the abs_home_path lint because we're in a build script,
    // which is explicitly allowed to use absolute paths
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    println!(
        "cargo:warning=Using manifest dir: {}",
        manifest_dir.display()
    );
}
