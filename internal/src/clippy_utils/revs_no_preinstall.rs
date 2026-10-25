use super::{
    file_at_revision, package_version_from_contents, repository::clippy_repository,
    toolchain_channel_from_contents,
};
use crate::git2::{Commit, Oid, Repository};
use anyhow::{Context, Result};
use semver::Version;
use std::{path::Path, rc::Rc};

#[derive(Debug, Eq, PartialEq)]
pub struct Rev {
    pub version: Version,
    pub channel: String,
    pub oid: Oid,
}

impl Rev {
    fn from_commit(repository: &Repository, commit: &Commit<'_>) -> Result<Self> {
        let oid = commit.id();
        let manifest = file_at_revision(repository, oid, Path::new("clippy_utils/Cargo.toml"))?;
        let version = package_version_from_contents(&manifest)?;
        // Match `read_rust_toolchain_file`'s precedence, including failing rather than
        // falling back if a present `rust-toolchain` file is malformed.
        let tree = commit.tree()?;
        let filename = if tree.get_name("rust-toolchain").is_some() {
            "rust-toolchain"
        } else {
            "rust-toolchain.toml"
        };
        let toolchain = file_at_revision(repository, oid, Path::new(filename))?;
        let channel = toolchain_channel_from_contents(&toolchain)?;
        Ok(Self {
            version,
            channel,
            oid,
        })
    }
}

pub struct Revs {
    repository: Rc<Repository>,
}

enum Filter {
    Version,
    Channel,
}

pub struct RevIter<'revs> {
    repository: Rc<Repository>,
    commit: Commit<'revs>,
    curr_rev: Option<Rev>,
    filter: Filter,
}

impl Revs {
    pub fn new(quiet: bool) -> Result<Self> {
        let repository = clippy_repository(quiet)?;

        Ok(Self { repository })
    }

    pub fn version_iter(&self) -> Result<RevIter<'_>> {
        self.iter(Filter::Version)
    }

    pub fn channel_iter(&self) -> Result<RevIter<'_>> {
        self.iter(Filter::Channel)
    }

    #[allow(clippy::iter_not_returning_iterator)]
    fn iter(&self, filter: Filter) -> Result<RevIter<'_>> {
        // Start from the branch used by `clippy_repository`, independently of the checkout.
        let commit = self
            .repository
            .find_reference("refs/heads/master")
            .and_then(|reference| reference.peel_to_commit())
            .with_context(|| "Could not resolve Clippy's local `master` branch")?;
        let rev = Rev::from_commit(&self.repository, &commit)?;
        Ok(RevIter {
            repository: self.repository.clone(),
            commit,
            curr_rev: Some(rev),
            filter,
        })
    }
}

impl Iterator for RevIter<'_> {
    type Item = Result<Rev>;

    // smoelius: I think it is okay to ignore the `non_local_effect_before_unhandled_error` warning
    // here. If `self.commit` were not updated, the same commits would be traversed the next time
    // `next` was called.
    #[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_unhandled_error))]
    fn next(&mut self) -> Option<Self::Item> {
        (|| -> Result<Option<Rev>> {
            let mut prev_rev: Option<Rev> = None;
            loop {
                let curr_rev = if let Some(rev) = self.curr_rev.take() {
                    rev
                } else {
                    // smoelius: Note that this is not `git log`'s default behavior. Rather, this
                    // behavior corresponds to:
                    //   git log --first-parent
                    let commit = if let Some(commit) = self.commit.parents().next() {
                        self.commit = commit.clone();
                        commit
                    } else {
                        return Ok(None);
                    };
                    Rev::from_commit(&self.repository, &commit)?
                };
                #[allow(clippy::collapsible_if)]
                if let Some(prev_rev) = prev_rev {
                    if match self.filter {
                        Filter::Version => prev_rev.version != curr_rev.version,
                        Filter::Channel => prev_rev.channel != curr_rev.channel,
                    } {
                        self.curr_rev = Some(curr_rev);
                        return Ok(Some(prev_rev));
                    }
                }
                prev_rev = Some(curr_rev);
            }
        })()
        .transpose()
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::git2::{IndexAddOption, RepositoryInitOptions, Signature, build::RepoBuilder};
    use std::{
        fs::{create_dir_all, read, read_to_string, remove_file, write},
        str::FromStr,
        sync::LazyLock,
    };
    use tempfile::{TempDir, tempdir};

    // Each filesystem mutation is confined to this test's temporary directory.
    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    #[test]
    fn local_searches_leave_checkout_unchanged() {
        let (dir, repository, commits) = local_repository();
        repository.set_head_detached(commits[1]).unwrap();
        let manifest = dir.path().join("clippy_utils/Cargo.toml");
        write(&manifest, "uncommitted, invalid TOML").unwrap();
        let index = read(repository.path().join("index")).unwrap();
        let head = read(repository.path().join("HEAD")).unwrap();
        let revs = Revs {
            repository: Rc::new(repository),
        };

        // Independent searches may overlap and may be repeated after an early exit.
        let mut versions = revs.version_iter().unwrap();
        let mut channels = revs.channel_iter().unwrap();
        assert_eq!(commits[3], versions.next().unwrap().unwrap().oid);
        assert_eq!(commits[3], channels.next().unwrap().unwrap().oid);
        assert_eq!(commits[2], channels.next().unwrap().unwrap().oid);
        assert_eq!(commits[1], versions.next().unwrap().unwrap().oid);
        assert_eq!(commits[1], channels.next().unwrap().unwrap().oid);
        assert!(versions.next().is_none());
        assert!(channels.next().is_none());
        assert_eq!(
            commits[3],
            revs.version_iter().unwrap().next().unwrap().unwrap().oid
        );
        // Symbol reads also use the selected commit, independent of the working tree.
        assert_eq!(
            "symbols at nightly-2025-05-03",
            file_at_revision(
                &revs.repository,
                commits[2],
                Path::new("clippy_utils/src/sym.rs")
            )
            .unwrap()
        );
        assert_eq!(head, read(revs.repository.path().join("HEAD")).unwrap());
        assert_eq!(index, read(revs.repository.path().join("index")).unwrap());
        assert_eq!(
            "uncommitted, invalid TOML",
            read_to_string(manifest).unwrap()
        );
        assert_eq!(
            "symbols at nightly-2025-05-04",
            read_to_string(dir.path().join("clippy_utils/src/sym.rs")).unwrap()
        );
    }

    // Each filesystem mutation is confined to this test's temporary directory.
    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    #[test]
    fn local_searches_support_bare_repositories() {
        let (dir, _, commits) = local_repository();
        let bare_dir = tempdir().unwrap();
        let repository = RepoBuilder::new()
            .bare(true)
            .clone(dir.path().to_str().unwrap(), bare_dir.path())
            .unwrap();
        assert!(repository.workdir().is_none());
        let revs = Revs {
            repository: Rc::new(repository),
        };
        let revisions = revs
            .channel_iter()
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            vec![commits[3], commits[2], commits[1]],
            revisions.iter().map(|rev| rev.oid).collect::<Vec<_>>()
        );
        // The legacy file takes precedence at this historical revision.
        assert_eq!("nightly-2025-05-02", revisions[2].channel);
        assert_eq!(
            "symbols at nightly-2025-05-02",
            file_at_revision(
                &revs.repository,
                commits[1],
                Path::new("clippy_utils/src/sym.rs")
            )
            .unwrap()
        );
    }

    // Each filesystem mutation is confined to this test's temporary directory.
    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    #[test]
    fn local_search_errors_leave_checkout_unchanged() {
        let (dir, repository, _) = local_repository();
        // A malformed legacy file must not fall back to the valid TOML file.
        write(dir.path().join("rust-toolchain"), "invalid TOML!").unwrap();
        let oid = commit_files(&repository);
        let revs = Revs {
            repository: Rc::new(repository),
        };
        assert!(revs.version_iter().is_err());
        assert_eq!(Some(oid), revs.repository.head().unwrap().target());
        assert!(file_at_revision(&revs.repository, oid, Path::new("missing")).is_err());
        // Also exercise an error reached after traversal has begun.
        remove_file(dir.path().join("rust-toolchain")).unwrap();
        commit_files(&revs.repository);
        let mut iter = revs.channel_iter().unwrap();
        assert!(iter.next().unwrap().is_err());
        assert_eq!(
            "symbols at nightly-2025-05-04",
            read_to_string(dir.path().join("clippy_utils/src/sym.rs")).unwrap()
        );
        assert_eq!(
            revs.repository
                .find_reference("refs/heads/master")
                .unwrap()
                .target(),
            revs.repository.head().unwrap().target()
        );
    }

    fn local_repository() -> (TempDir, Repository, Vec<Oid>) {
        let dir = tempdir().unwrap();
        let mut options = RepositoryInitOptions::new();
        options.initial_head("master");
        let repository = Repository::init_opts(dir.path(), &options).unwrap();
        create_dir_all(dir.path().join("clippy_utils/src")).unwrap();
        let mut commits = Vec::new();
        for (day, version) in [(1, "0.1.80"), (2, "0.1.81"), (3, "0.1.81"), (4, "0.1.82")] {
            let channel = format!("nightly-2025-05-{day:02}");
            write(
                dir.path().join("clippy_utils/Cargo.toml"),
                format!("[package]\nversion = \"{version}\"\n"),
            )
            .unwrap();
            write(
                dir.path().join("rust-toolchain.toml"),
                format!("[toolchain]\nchannel = \"{channel}\"\n"),
            )
            .unwrap();
            if day == 2 {
                write(
                    dir.path().join("rust-toolchain"),
                    format!("[toolchain]\nchannel = \"{channel}\"\n"),
                )
                .unwrap();
                write(
                    dir.path().join("rust-toolchain.toml"),
                    "[toolchain]\nchannel = \"nightly-2099-01-01\"\n",
                )
                .unwrap();
            } else if dir.path().join("rust-toolchain").exists() {
                remove_file(dir.path().join("rust-toolchain")).unwrap();
            }
            write(
                dir.path().join("clippy_utils/src/sym.rs"),
                format!("symbols at {channel}"),
            )
            .unwrap();
            commits.push(commit_files(&repository));
        }
        (dir, repository, commits)
    }

    fn commit_files(repository: &Repository) -> Oid {
        let mut index = repository.index().unwrap();
        index.add_all(["*"], IndexAddOption::DEFAULT, None).unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        index.write().unwrap();
        let parent = repository
            .head()
            .ok()
            .map(|head| head.peel_to_commit().unwrap());
        let parents = parent.iter().collect::<Vec<_>>();
        let signature = Signature::now("Test", "test@example.com").unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "test",
                &tree,
                &parents,
            )
            .unwrap()
    }

    static VERSION_EXAMPLES: LazyLock<[Rev; 6]> = LazyLock::new(|| {
        [
            Rev {
                version: Version::from_str("0.1.65").unwrap(),
                channel: "nightly-2022-08-11".to_owned(),
                oid: Oid::from_str("2b2190cb5667cdd276a24ef8b9f3692209c54a89").unwrap(),
            },
            Rev {
                version: Version::from_str("0.1.64").unwrap(),
                channel: "nightly-2022-06-30".to_owned(),
                oid: Oid::from_str("0cb0f7636851f9fcc57085cf80197a2ef6db098f").unwrap(),
            },
            // smoelius: 0.1.62 and 0.1.63 omitted (for no particular reason).
            Rev {
                version: Version::from_str("0.1.61").unwrap(),
                channel: "nightly-2022-02-24".to_owned(),
                oid: Oid::from_str("7b2896a8fc9f0b275692677ee6d2d66a7cbde16a").unwrap(),
            },
            Rev {
                version: Version::from_str("0.1.60").unwrap(),
                channel: "nightly-2022-01-13".to_owned(),
                oid: Oid::from_str("97a5daa65908e59744e2bc625b14849352231c75").unwrap(),
            },
            Rev {
                version: Version::from_str("0.1.59").unwrap(),
                channel: "nightly-2021-12-02".to_owned(),
                oid: Oid::from_str("392b0c5c25ddbd36e4dc480afcf70ed01dce352d").unwrap(),
            },
            Rev {
                version: Version::from_str("0.1.58").unwrap(),
                channel: "nightly-2021-10-21".to_owned(),
                oid: Oid::from_str("91496c2ac6abf6454c413bb23e8becf6b6dc20ea").unwrap(),
            },
        ]
    });

    // smoelius: To reiterate a comment from package_options/mod.rs, the `find` below is an
    // iterative search and it is causing this test to become increasingly slow. As of this writing,
    // the test takes around two minutes. The search should be a binary search.
    #[test]
    fn versions() {
        for example in &*VERSION_EXAMPLES {
            let revs = Revs::new(false).unwrap();
            let mut iter = revs.version_iter().unwrap();
            let rev = iter
                .find(|rev| {
                    rev.as_ref()
                        .map_or(true, |rev| rev.version <= example.version)
                })
                .unwrap()
                .unwrap();
            assert_eq!(*example, rev);
        }
    }

    static CHANNEL_EXAMPLES: LazyLock<[Rev; 2]> = LazyLock::new(|| {
        [
            Rev {
                version: Version::from_str("0.1.89").unwrap(),
                channel: "nightly-2025-05-14".to_owned(),
                oid: Oid::from_str("0450db33a5d8587f7c1d4b6d233dac963605766b").unwrap(),
            },
            Rev {
                version: Version::from_str("0.1.88").unwrap(),
                channel: "nightly-2025-05-01".to_owned(),
                oid: Oid::from_str("03a5b6b976ac121f4233775c49a4bce026065b47").unwrap(),
            },
        ]
    });

    #[test]
    fn channels() {
        for example in &*CHANNEL_EXAMPLES {
            let revs = Revs::new(false).unwrap();
            let mut iter = revs.channel_iter().unwrap();
            let rev = iter
                .find(|rev| {
                    rev.as_ref()
                        .map_or(true, |rev| rev.channel <= example.channel)
                })
                .unwrap()
                .unwrap();
            assert_eq!(*example, rev);
        }
    }
}
