use crate::{clone, env, git2::Repository};
use anyhow::{Context, Result};
use std::{cell::RefCell, path::PathBuf, rc::Rc};
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

        let repository = if let Some(path) = std::env::var_os(env::DYLINT_CLIPPY_REPO) {
            anyhow::ensure!(
                !path.is_empty(),
                "{} must not be empty",
                env::DYLINT_CLIPPY_REPO
            );
            let path = PathBuf::from(path);
            // EricLiu2000: Revision searches change HEAD and the worktree. Keep the supplied
            // repository unchanged, and clone only from its local object database.
            let source = Repository::open(&path).with_context(|| {
                format!(
                    "{} must point to a Git repository: `{}`",
                    env::DYLINT_CLIPPY_REPO,
                    path.display()
                )
            })?;
            let source_path = source.path().to_str().ok_or_else(|| {
                anyhow::anyhow!(
                    "Clippy Git repository path is not valid UTF-8: `{}`",
                    source.path().display()
                )
            })?;
            Repository::clone(source_path, &tempdir).with_context(|| {
                format!(
                    "Could not clone local Clippy repository `{}`",
                    path.display()
                )
            })?
        } else {
            clone(RUST_CLIPPY_URL, "master", tempdir.path(), quiet)?
        };
        let repository = Rc::new(repository);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommandExt;
    use std::{
        fs::{create_dir, read_to_string, write},
        path::Path,
        process::Command,
    };

    const DYLINT_TEST_REPOSITORY: &str = "DYLINT_TEST_REPOSITORY";

    #[cfg_attr(
        dylint_lib = "general",
        allow(
            non_thread_safe_call_in_test,
            reason = "All writes use a private temporary directory"
        )
    )]
    #[test]
    fn supplied_repository() {
        if std::env::var_os(DYLINT_TEST_REPOSITORY).is_some() {
            let repository = clippy_repository(true).unwrap();
            let head = repository
                .head()
                .and_then(|reference| reference.peel_to_commit())
                .unwrap();
            assert_eq!("second", head.message().unwrap());
            let parent = head.parent(0).unwrap();
            repository.checkout_tree(parent.as_object(), None).unwrap();
            repository.set_head_detached(parent.id()).unwrap();
            assert_eq!(
                "first",
                read_to_string(repository.workdir().unwrap().join("source")).unwrap()
            );
            return;
        }

        let dir = tempdir().unwrap();
        let repository = Repository::init(&dir).unwrap();
        let signature = git2::Signature::now("Test", "test@example.com").unwrap();
        let commit = |contents: &str, parents: &[&git2::Commit<'_>]| {
            write(dir.path().join("source"), contents).unwrap();
            let mut index = repository.index().unwrap();
            index.add_path(Path::new("source")).unwrap();
            let oid = index.write_tree().unwrap();
            let tree = repository.find_tree(oid).unwrap();
            repository
                .commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    contents,
                    &tree,
                    parents,
                )
                .unwrap()
        };
        let first = commit("first", &[]);
        let second = commit("second", &[&repository.find_commit(first).unwrap()]);
        repository.set_head_detached(second).unwrap();

        run_child(dir.path(), "supplied_repository");
    }

    #[cfg_attr(
        dylint_lib = "general",
        allow(
            non_thread_safe_call_in_test,
            reason = "All writes use a private temporary directory"
        )
    )]
    #[test]
    fn invalid_supplied_repository() {
        if std::env::var_os(DYLINT_TEST_REPOSITORY).is_some() {
            let error = clippy_repository(true).err().unwrap();
            assert!(format!("{error:#}").contains(env::DYLINT_CLIPPY_REPO));
            return;
        }

        let dir = tempdir().unwrap();
        let file = dir.path().join("file");
        write(&file, "not a repository").unwrap();
        let directory = dir.path().join("directory");
        create_dir(&directory).unwrap();
        for path in [
            Path::new(""),
            &dir.path().join("missing"),
            &file,
            &directory,
        ] {
            run_child(path, "invalid_supplied_repository");
        }
    }

    // EricLiu2000: Isolate environment inputs and the thread-local repository cache for each case.
    fn run_child(path: &Path, test: &str) {
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!("clippy_utils::repository::tests::{test}"),
                "--nocapture",
            ])
            .env(env::DYLINT_CLIPPY_REPO, path)
            .env(DYLINT_TEST_REPOSITORY, "1")
            .env(env::PATH, "")
            .logged_output(true)
            .unwrap();
    }
}
