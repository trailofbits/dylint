/// This is a simple test program for the abs_home_path lint.
/// It exists to test that the lint correctly allows absolute paths in build scripts.
fn main() {
    println!("Testing abs_home_path lint's build script allowance");

    // Note: The actual test happens in the build.rs file, which references
    // an absolute path that would normally trigger the lint
}
