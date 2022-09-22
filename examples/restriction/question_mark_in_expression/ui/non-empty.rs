fn main() -> Result<(), std::io::Error> {
    if !std::fs::read_to_string("Cargo.toml")?.is_empty() {
        println!("Cargo.toml is non-empty.");
    }

    if String::new() != std::fs::read_to_string("Cargo.lock")? {
        println!("Cargo.lock is non-empty.");
    }

    Ok(())
}
