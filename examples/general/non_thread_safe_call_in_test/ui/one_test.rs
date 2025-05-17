fn main() {}

#[test]
fn set_var() {
    unsafe {
        std::env::set_var("KEY", "VALUE");
    }
    std::process::Command::new("env").status().unwrap();
}
