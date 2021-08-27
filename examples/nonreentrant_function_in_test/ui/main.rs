fn main() {
    std::env::set_var("KEY", "VALUE");
    std::process::Command::new("env").status().unwrap();
}

#[test]
fn set_var() {
    std::env::set_var("KEY", "VALUE");
    std::process::Command::new("env").status().unwrap();
}

#[test]
fn command_env() {
    std::process::Command::new("env")
        .env("KEY", "VALUE")
        .status()
        .unwrap();
}

#[cfg(test)]
mod test {
    fn set_var() {
        std::env::set_var("KEY", "VALUE");
        std::process::Command::new("env").status().unwrap();
    }
}
