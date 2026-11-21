fn main() {
    let _ = std::env::var("RUSTFLAGS");
    std::env::remove_var("RUSTFALGS");
    std::env::set_var("RUSTFALGS", "-D warnings");
}
