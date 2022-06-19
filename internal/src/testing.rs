use anyhow::Result;
use std::path::Path;

pub fn new_template(path: &Path) -> Result<()> {
    crate::packaging::new_template(path)?;
    crate::packaging::use_local_packages(path)?;
    Ok(())
}
