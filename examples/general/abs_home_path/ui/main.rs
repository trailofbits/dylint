fn main() {
    let _ = env!("CARGO");
    let _ = env!("CARGO_MANIFEST_DIR");
    let _ = option_env!("CARGO");
    let _ = option_env!("CARGO_MANIFEST_DIR");
}

#[test]
fn test() {
    let _ = env!("CARGO");
}
