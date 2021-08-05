use anyhow::{anyhow, Result};
use std::{
    fs::{read_to_string, write},
    path::Path,
};

pub fn find_and_replace<I>(path: &Path, commands: I) -> Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let before = read_to_string(path)?;
    let after =
        sedregex::find_and_replace(&before, commands).map_err(|error| anyhow!("{}", error))?;
    write(path, after.as_bytes()).map_err(Into::into)
}
