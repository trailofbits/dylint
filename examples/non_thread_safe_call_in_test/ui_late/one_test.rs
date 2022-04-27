fn main() {}

#[test]
fn set_var() {
    std::env::set_var("KEY", "VALUE");
    std::process::Command::new("env").status().unwrap();
}
