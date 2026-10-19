use crate::{
    clone,
    git2::{Oid, Repository},
};
use anyhow::{Context, Result};
use std::{cell::RefCell, path::Path, rc::Rc};
use tempfile::{TempDir, tempdir};

const RUST_CLIPPY_URL: &str = "https://github.com/rust-lang/rust-clippy";

// smoelius: `thread_local!` because `git2::Repository` cannot be shared between threads safely.
thread_local! {
    static TMPDIR_AND_REPOSITORY: RefCell<Option<(TempDir, Rc<Repository>)>> = const { RefCell::new(None) };
}

pub fn clippy_repository(quiet: bool) -> Result<Rc<Repository>> {
    TMPDIR_AND_REPOSITORY.with_borrow_mut(|cell| {
        if let Some((_, repository)) = cell {
            return Ok(repository.clone());
        }

        let tempdir = tempdir().with_context(|| "`tempdir` failed")?;

        let repository = clone(RUST_CLIPPY_URL, "master", tempdir.path(), quiet).map(Rc::new)?;

        cell.replace((tempdir, repository.clone()));

        Ok(repository)
    })
}

/// Reads a UTF-8 file from a commit without changing HEAD or the working tree.
pub fn file_at_revision(repository: &Repository, rev: Oid, path: &Path) -> Result<String> {
    (|| -> Result<String> {
        let commit = repository.find_commit(rev)?;
        let tree = commit.tree()?;
        let entry = tree.get_path(path)?;
        let blob = repository.find_blob(entry.id())?;
        Ok(std::str::from_utf8(blob.content())?.to_owned())
    })()
    .with_context(|| format!("Could not read `{}` at revision `{rev}`", path.display()))
}

pub fn parse_as_nightly(channel: &str) -> Option<[u32; 3]> {
    channel.strip_prefix("nightly-").and_then(parse_date)
}

fn parse_date(date_str: &str) -> Option<[u32; 3]> {
    date_str
        .split('-')
        .map(str::parse::<u32>)
        .map(Result::ok)
        .collect::<Option<Vec<_>>>()
        .map(<[u32; 3]>::try_from)
        .and_then(Result::ok)
}
