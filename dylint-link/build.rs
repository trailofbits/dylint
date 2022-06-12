use dylint_internal::env;

fn main() {
    println!("cargo:rustc-env=TARGET={}", env::var(env::TARGET).unwrap());
}
