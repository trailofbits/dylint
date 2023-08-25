pub fn greet() -> Result<(), std::str::Utf8Error> {
    println!("{}", std::str::from_utf8(b"Hello, world!")?);
    Ok(())
}
