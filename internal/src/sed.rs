use anyhow::{Context, Result};
use regex::Regex;
use std::{
    fs::{read_to_string, write},
    path::Path,
};

pub fn find_and_replace<R>(path: &Path, re: &str, replacement: R) -> Result<()>
where
    R: AsRef<str>,
{
    let before = read_to_string(path)
        .with_context(|| format!("`read_to_string` failed for `{}`", path.to_string_lossy()))?;
    let re = Regex::new(re)?;
    let after = re.replace_all(&before, replacement.as_ref());
    write(path, after.as_bytes()).map_err(Into::into)
}
