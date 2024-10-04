#![allow(dead_code)]

use anyhow::Context;
use std::{fs::File, io, path::PathBuf};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("io error")]
    Io(#[from] io::Error),
    #[error("failed to open {0:?}")]
    OpenFailed(PathBuf, #[source] io::Error),
}

fn main() {}

fn foo() -> anyhow::Result<()> {
    let _ = File::open("/nonexistent")?;
    Ok(())
}

fn foo_with_context() -> anyhow::Result<()> {
    let _ = File::open("/nonexistent").with_context(|| "could not open `/nonexistent`")?;
    Ok(())
}

fn bar() -> Result<(), Error> {
    let _ = File::open("/nonexistent")?;
    Ok(())
}

fn bar_with_context() -> Result<(), Error> {
    let _ = File::open("/nonexistent")
        .map_err(|error| Error::OpenFailed(PathBuf::from("/nonexistent"), error))?;
    Ok(())
}

fn baz() -> io::Result<()> {
    let _ = File::open("/nonexistent")?;
    Ok(())
}
