use dylint_internal::env;
use std::process;

fn main() {
    match env::var(env::TARGET) {
        Ok(target) => println!("cargo:rustc-env=TARGET={target}"),
        Err(err) => {
            eprintln!("Error getting target: {err}");
            process::exit(1);
        }
    }
}
