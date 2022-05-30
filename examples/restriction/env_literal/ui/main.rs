fn main() {
    let _ = std::env::var("RUSTFLAGS");
    std::env::remove_var("RUSTFALGS");
}
