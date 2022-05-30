fn main() {}

#[cfg(test)]
mod test {
    #[test]
    fn foo() {
        set_var();
    }

    #[test]
    fn bar() {
        set_var();
    }

    fn set_var() {
        std::env::set_var("KEY", "VALUE");
        std::process::Command::new("env").status().unwrap();
    }
}
