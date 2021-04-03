use anyhow::Result;
use git2::{Oid, Repository, ResetType};
use std::path::Path;

// smoelius: This function performs a hard reset instead of a checkout. It works but it is
// technically broken.
pub fn checkout(url: &str, rev: &str, path: &Path) -> Result<()> {
    let oid = Oid::from_str(rev)?;

    let repository = Repository::clone(url, path)?;
    let object = repository.find_object(oid, None)?;
    repository.reset(&object, ResetType::Hard, None)?;

    Ok(())
}
