use anyhow::{Context, Result};
use git2::Repository;
use if_chain::if_chain;
use std::path::Path;

pub fn clone(url: &str, refname: &str, path: &Path) -> Result<Repository> {
    let repository = Repository::clone(url, path)?;

    checkout(&repository, refname)?;

    Ok(repository)
}

// smoelius: `checkout` is based on: https://stackoverflow.com/a/67240436
pub fn checkout(repository: &Repository, refname: &str) -> Result<()> {
    let (object, reference) = repository
        .revparse_ext(refname)
        .with_context(|| format!("`revparse_ext` failed for `{}`", refname))?;

    repository
        .checkout_tree(&object, None)
        .with_context(|| format!("`checkout_tree` failed for `{:?}`", object))?;

    if_chain! {
        if let Some(reference) = reference;
        if let Some(refname) = reference.name();
        then {
            repository
                .set_head(refname)
                .with_context(|| format!("`set_head` failed for `{}`", refname))?;
        } else {
            repository
                .set_head_detached(object.id())
                .with_context(|| format!("`set_head_detached` failed for `{}`", object.id()))?;
        }
    }

    Ok(())
}
