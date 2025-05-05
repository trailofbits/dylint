fn main() {
    println!("Testing abs_home_path lint's test context allowance");
    non_test_function_with_home_path();
}

fn non_test_function_with_home_path() {
    // This should trigger the lint
    let _ = env!("CARGO_MANIFEST_DIR");
    let _ = option_env!("CARGO");
}

#[test]
fn test_function_with_home_path() {
    // This should NOT trigger the lint
    let _ = env!("CARGO_MANIFEST_DIR");
    let _ = option_env!("CARGO");
}
