// This is a test program for verifying that the abs_home_path lint
// correctly allows absolute paths in build scripts, while still catching them in regular code.
fn main() {
    println!("Testing abs_home_path lint's build script allowance");

    // The actual test is in build.rs, which references an absolute path
    // that would normally trigger the lint, but should be allowed in build scripts
}
