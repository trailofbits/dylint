use super::common::clippy_repository;
use anyhow::{ Context, Result, anyhow };
use dylint_internal::git2::{ Oid, Repository };
use std::{ path::Path, time::Instant };
use toml::Value;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Rev {
    pub version: String,
    pub channel: String,
    pub oid: Oid,
}

pub struct Revs {
    versions: Vec<Rev>,
}

impl Revs {
    pub fn new(quiet: bool) -> Result<Self> {
        let start = Instant::now();
        let repository = clippy_repository(quiet)?;
        let versions = Self::build_version_index(&repository, quiet)?;

        if !quiet {
            eprintln!("Revs initialization took: {:?}", start.elapsed());
        }

        Ok(Self { versions })
    }

    fn build_version_index(repo: &Repository, quiet: bool) -> Result<Vec<Rev>> {
        let start = Instant::now();
        let mut revwalk = repo.revwalk().context("Failed to create revision walker")?;
        revwalk.push_head().context("Failed to push HEAD to revision walker")?;

        let mut versions = Vec::new();
        let mut prev_version = String::new();

        for oid in revwalk {
            let oid = oid.context("Failed to get commit OID")?;
            let commit = repo.find_commit(oid).context("Failed to find commit")?;
            let tree = commit.tree().context("Failed to get commit tree")?;

            // Try to get both Cargo.toml and rust-toolchain files
            if let Ok(cargo_entry) = tree.get_path(Path::new("Cargo.toml")) {
                let cargo_blob = repo
                    .find_blob(cargo_entry.id())
                    .context("Failed to find Cargo.toml blob")?;
                let cargo_content = std::str
                    ::from_utf8(cargo_blob.content())
                    .context("Failed to parse Cargo.toml content as UTF-8")?;

                // Parse version from Cargo.toml content
                let cargo_toml: Value = toml
                    ::from_str(cargo_content)
                    .context("Failed to parse Cargo.toml as TOML")?;
                let version = cargo_toml
                    .get("package")
                    .and_then(|p| p.get("version"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing or invalid version in Cargo.toml"))?
                    .to_string();

                // Try to get channel from rust-toolchain file
                let channel = if
                    let Ok(toolchain_entry) = tree.get_path(Path::new("rust-toolchain"))
                {
                    let toolchain_blob = repo.find_blob(toolchain_entry.id())?;
                    let toolchain_content = std::str
                        ::from_utf8(toolchain_blob.content())
                        .context("Failed to parse rust-toolchain content as UTF-8")?;
                    toolchain_content.trim().to_string()
                } else {
                    // Fallback to a default channel if rust-toolchain is not found
                    "nightly".to_string()
                };

                // Only store if version changed
                if version != prev_version {
                    versions.push(Rev {
                        version: version.clone(),
                        channel,
                        oid,
                    });
                    prev_version = version;
                }
            }
        }

        // Sort versions for binary search
        versions.sort_by(|a, b| a.version.cmp(&b.version));

        if !quiet {
            eprintln!(
                "Built version index with {} versions in {:?}",
                versions.len(),
                start.elapsed()
            );
        }

        Ok(versions)
    }

    pub fn find_version(&self, target_version: &str) -> Result<Option<&Rev>> {
        let start = Instant::now();
        let result = self.versions
            .binary_search_by(|rev| rev.version.as_str().cmp(target_version))
            .ok()
            .map(|idx| &self.versions[idx]);

        eprintln!("Version search for {} took {:?}", target_version, start.elapsed());

        Ok(result)
    }

    #[allow(clippy::iter_not_returning_iterator)]
    pub fn iter(&self) -> Result<RevIter> {
        Ok(RevIter {
            versions: &self.versions,
            current_idx: 0,
        })
    }
}

pub struct RevIter<'a> {
    versions: &'a [Rev],
    current_idx: usize,
}

impl<'a> Iterator for RevIter<'a> {
    type Item = Result<Rev>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_idx >= self.versions.len() {
            return None;
        }

        let rev = self.versions[self.current_idx].clone();
        self.current_idx += 1;
        Some(Ok(rev))
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use super::*;
    use std::sync::LazyLock;

    static EXAMPLES: LazyLock<[Rev; 6]> = LazyLock::new(|| {
        [
            Rev {
                version: "0.1.65".to_owned(),
                channel: "nightly-2022-08-11".to_owned(),
                oid: Oid::from_str("2b2190cb5667cdd276a24ef8b9f3692209c54a89").unwrap(),
            },
            Rev {
                version: "0.1.64".to_owned(),
                channel: "nightly-2022-06-30".to_owned(),
                oid: Oid::from_str("0cb0f7636851f9fcc57085cf80197a2ef6db098f").unwrap(),
            },
            Rev {
                version: "0.1.61".to_owned(),
                channel: "nightly-2022-02-24".to_owned(),
                oid: Oid::from_str("7b2896a8fc9f0b275692677ee6d2d66a7cbde16a").unwrap(),
            },
            Rev {
                version: "0.1.60".to_owned(),
                channel: "nightly-2022-01-13".to_owned(),
                oid: Oid::from_str("97a5daa65908e59744e2bc625b14849352231c75").unwrap(),
            },
            Rev {
                version: "0.1.59".to_owned(),
                channel: "nightly-2021-12-02".to_owned(),
                oid: Oid::from_str("392b0c5c25ddbd36e4dc480afcf70ed01dce352d").unwrap(),
            },
            Rev {
                version: "0.1.58".to_owned(),
                channel: "nightly-2021-10-21".to_owned(),
                oid: Oid::from_str("91496c2ac6abf6454c413bb23e8becf6b6dc20ea").unwrap(),
            },
        ]
    });

    #[test]
    fn examples() {
        let start = Instant::now();

        let revs = Revs::new(false).unwrap();
        let init_duration = start.elapsed();
        println!("Initialization took: {:?}", init_duration);

        for example in &*EXAMPLES {
            let search_start = Instant::now();
            let found = revs.find_version(&example.version).unwrap().unwrap();
            println!("Search for version {} took: {:?}", example.version, search_start.elapsed());
            assert_eq!(found.version, example.version);
            assert_eq!(found.channel, example.channel);
            assert_eq!(found.oid, example.oid);
        }

        println!("Total test duration: {:?}", start.elapsed());
    }
}
