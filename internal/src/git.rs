use anyhow::Result;
use git2::Repository;
use if_chain::if_chain;
use std::path::Path;

// smoelius: `checkout` is based on: https://stackoverflow.com/a/67240436
pub fn checkout(url: &str, refname: &str, path: &Path) -> Result<()> {
    let repository = Repository::clone(url, path)?;

    let (object, reference) = repository.revparse_ext(refname)?;

    repository.checkout_tree(&object, None)?;

    if_chain! {
        if let Some(reference) = reference;
        if let Some(refname) = reference.name();
        then {
            repository.set_head(refname)?;
        } else {
            repository.set_head_detached(object.id())?;
        }
    }

    Ok(())
}
