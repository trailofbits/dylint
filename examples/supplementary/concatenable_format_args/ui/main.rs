fn main() {
    should_lint();
    should_not_lint();
}

// Cases that SHOULD trigger the lint (`env!` and literal arguments with `Display` formatting)
fn should_lint() {
    let _ = format!("{}/**/*.md", env!("CARGO_MANIFEST_DIR"));
    let _ = format!("{}/**/*.yml", env!("CARGO_MANIFEST_DIR"));
    let _ = format!("!{}/target/**", env!("CARGO_MANIFEST_DIR"));
    let _ = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), env!("CARGO_PKG_NAME"));
    let _ = format!("{}-{}", env!("CARGO_PKG_NAME"), "lint");
    let _ = std::format!("{}/src", env!("CARGO_MANIFEST_DIR"));
    let _ = format!("{}/src", std::env!("CARGO_MANIFEST_DIR"));

    // Trailing comma after the last argument.
    let _ = format!("{}/trail", env!("CARGO_MANIFEST_DIR"),);

    // Three arguments mixing string literals and `env!` invocations in non-trivial order.
    let _ = format!(
        "{}-{}-{}",
        "prefix",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
}

// Cases that SHOULD NOT trigger the lint
fn should_not_lint() {
    // String-only formatting without `env!`
    let _ = format!("hello {}", "world");

    // Raw format strings
    let _ = format!(r"{}/**/*.md", env!("CARGO_MANIFEST_DIR"));

    // Unsupported non-`env!` arguments
    let path = "path";
    let _ = format!("{}", path);

    // Non-`Display` formatting
    let _ = format!("manifest: {:?}", env!("CARGO_MANIFEST_DIR"));

    // Not `format!` macro
    println!("this is not format!");

    // `concat!` is already used; the lint should never fire on `concat!` itself.
    let _: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/foo");
}
