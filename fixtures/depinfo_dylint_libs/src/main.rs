use anyhow::Result;
use std::{
    io::{stdout, Write},
    str::from_utf8,
};

fn main() -> Result<()> {
    write!(stdout(), "{}", from_utf8(b"Hello, world!")?)?;
    Ok(())
}
