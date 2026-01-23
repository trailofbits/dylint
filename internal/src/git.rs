use crate::CommandExt;
use anyhow::{Context, Result};
use git2::Repository;
use std::{
    path::Path,
    process::{Command, Stdio},
};

// smoelius: I think this imitates Cargo's default behavior:
// https://doc.rust-lang.org/cargo/reference/config.html#netretry
const N_RETRIES: usize = 2;

// smoelius: I think I may have run into https://github.com/libgit2/libgit2/issues/5294 a few times,
// but I don't know of a good general-purpose solution. TODO: Investigate whether/how Cargo's
// wrappers handle this.
pub fn clone(url: &str, refname: &str, path: &Path, quiet: bool) -> Result<Repository> {
    let repository = if Command::new("git")
        .args(["--version"])
        .stdout(Stdio::null())
        .success()
        .is_ok()
    {
        clone_with_cli(url, path, quiet)
    } else {
        clone_with_git2(url, path, quiet)
    }?;

    checkout(&repository, refname)?;

    Ok(repository)
}

fn clone_with_cli(url: &str, path: &Path, quiet: bool) -> Result<Repository> {
    let mut command = Command::new("git");
    command.args(["clone", url, &path.to_string_lossy()]);
    if quiet {
        command.args(["--quiet"]);
    }
    command.success()?;

    Repository::open(path).map_err(Into::into)
}

fn clone_with_git2(url: &str, path: &Path, _quiet: bool) -> Result<Repository> {
    let mut result = Repository::clone(url, path);

    for _ in 0..N_RETRIES {
        if result.is_err() {
            result = Repository::clone(url, path);
        } else {
            break;
        }
    }

    result.map_err(Into::into)
}

// smoelius: `checkout` is based on: https://stackoverflow.com/a/67240436
pub fn checkout(repository: &Repository, refname: &str) -> Result<()> {
    let (object, reference) = repository
        .revparse_ext(refname)
        .with_context(|| format!("`revparse_ext` failed for `{refname}`"))?;

    repository
        .checkout_tree(&object, None)
        .with_context(|| format!("`checkout_tree` failed for `{object:?}`"))?;

    match reference.as_ref().and_then(|r| r.name()) {
        Some(refname) => {
            repository
                .set_head(refname)
                .with_context(|| format!("`set_head` failed for `{refname}`"))?;
        }
        None => {
            repository
                .set_head_detached(object.id())
                .with_context(|| format!("`set_head_detached` failed for `{}`", object.id()))?;
        }
    }

    Ok(())
}
