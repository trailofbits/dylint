use anyhow::Result;
use std::path::Path;

#[ctor::ctor]
fn init() {
    env_logger::init();
}

pub fn new_template(path: &Path) -> Result<()> {
    crate::packaging::new_template(path)?;
    crate::packaging::use_local_packages(path)?;
    Ok(())
}
