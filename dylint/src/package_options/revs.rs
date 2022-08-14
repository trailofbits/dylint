use super::clippy_utils::{channel, version};
use anyhow::{anyhow, Context, Result};
use dylint_internal::{
    clone,
    git2::{Commit, ObjectType, Repository},
};
use if_chain::if_chain;
use tempfile::{tempdir, TempDir};

const RUST_CLIPPY_URL: &str = "https://github.com/rust-lang/rust-clippy";

#[derive(Debug, Eq, PartialEq)]
pub struct Rev {
    pub version: String,
    pub channel: String,
    pub rev: String,
}

pub struct Revs {
    tempdir: TempDir,
    repository: Repository,
}

pub struct RevIter<'revs> {
    revs: &'revs Revs,
    commit: Commit<'revs>,
    curr_rev: Option<Rev>,
}

impl Revs {
    pub fn new() -> Result<Self> {
        let tempdir = tempdir().with_context(|| "`tempdir` failed")?;

        let repository = clone(RUST_CLIPPY_URL, "master", tempdir.path())?;

        Ok(Self {
            tempdir,
            repository,
        })
    }

    #[allow(clippy::iter_not_returning_iterator)]
    pub fn iter(&self) -> Result<RevIter> {
        let object = {
            let head = self.repository.head()?;
            let oid = head.target().ok_or_else(|| anyhow!("Could not get HEAD"))?;
            self.repository.find_object(oid, Some(ObjectType::Commit))?
        };
        let commit = object
            .as_commit()
            .ok_or_else(|| anyhow!("Object is not a commit"))?;
        let version = version(self.tempdir.path())?;
        let channel = channel(self.tempdir.path())?;
        let rev = commit.id().to_string();
        Ok(RevIter {
            revs: self,
            commit: commit.clone(),
            curr_rev: Some(Rev {
                version,
                channel,
                rev,
            }),
        })
    }
}

impl<'revs> Iterator for RevIter<'revs> {
    type Item = Result<Rev>;

    // smoelius: I think it is okay to ignore the `non_local_effect_before_error_return` warning
    // here. If `self.commit` were not updated, the same commits would be traversed the next time
    // `next` was called.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_error_return",
        allow(non_local_effect_before_error_return)
    )]
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
                    self.revs
                        .repository
                        .checkout_tree(commit.as_object(), None)
                        .with_context(|| {
                            format!("`checkout_tree` failed for `{:?}`", commit.as_object())
                        })?;
                    self.revs
                        .repository
                        .set_head_detached(commit.id())
                        .with_context(|| {
                            format!("`set_head_detached` failed for `{}`", commit.id())
                        })?;
                    let version = version(self.revs.tempdir.path())?;
                    let channel = channel(self.revs.tempdir.path())?;
                    let rev = commit.id().to_string();
                    Rev {
                        version,
                        channel,
                        rev,
                    }
                };
                if_chain! {
                    if let Some(prev_rev) = prev_rev;
                    if prev_rev.version != curr_rev.version;
                    then {
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

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref EXAMPLES: [Rev; 4] = [
            Rev {
                version: "0.1.61".to_owned(),
                channel: "nightly-2022-02-24".to_owned(),
                rev: "7b2896a8fc9f0b275692677ee6d2d66a7cbde16a".to_owned(),
            },
            Rev {
                version: "0.1.60".to_owned(),
                channel: "nightly-2022-01-13".to_owned(),
                rev: "97a5daa65908e59744e2bc625b14849352231c75".to_owned(),
            },
            Rev {
                version: "0.1.59".to_owned(),
                channel: "nightly-2021-12-02".to_owned(),
                rev: "392b0c5c25ddbd36e4dc480afcf70ed01dce352d".to_owned(),
            },
            Rev {
                version: "0.1.58".to_owned(),
                channel: "nightly-2021-10-21".to_owned(),
                rev: "91496c2ac6abf6454c413bb23e8becf6b6dc20ea".to_owned(),
            },
        ];
    }

    #[test]
    fn examples() {
        for example in EXAMPLES.iter() {
            let revs = Revs::new().unwrap();
            let mut iter = revs.iter().unwrap();
            let rev = iter
                .find(|rev| {
                    rev.as_ref()
                        .map_or(true, |rev| rev.version == example.version)
                })
                .unwrap()
                .unwrap();
            assert_eq!(rev, *example);
        }
    }
}
