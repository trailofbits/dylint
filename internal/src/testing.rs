use anyhow::Result;
use std::path::Path;

const DYLINT_TEMPLATE_URL: &str = "https://github.com/trailofbits/dylint-template";

pub fn clone_dylint_template(path: &Path) -> Result<()> {
    crate::clone(DYLINT_TEMPLATE_URL, "master", path)?;
    crate::packaging::isolate(path)?;
    crate::packaging::use_local_packages(path)?;
    crate::packaging::allow_unused_extern_crates(path)?;
    Ok(())
}
