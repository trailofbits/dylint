use crate::{clone, git2::Repository};
use anyhow::{Context, Result};
use std::{cell::RefCell, rc::Rc};
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
