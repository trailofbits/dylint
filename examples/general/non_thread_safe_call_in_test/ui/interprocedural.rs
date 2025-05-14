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
        unsafe {
            std::env::set_var("KEY", "VALUE");
        }
        std::process::Command::new("env").status().unwrap();
    }

    #[test]
    fn baz() {
        cargo_arg_run();
        cargo_args_run();
    }

    fn cargo_arg_run() {
        std::process::Command::new("cargo")
            .arg("run")
            .status()
            .unwrap();
    }

    fn cargo_args_run() {
        std::process::Command::new("cargo")
            .args(["run"])
            .status()
            .unwrap();
    }
}
