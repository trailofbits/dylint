use super::common::clippy_repository;
use anyhow::{Context, Result, anyhow};
use dylint_internal::git2::{Commit, Oid, Repository, Sort};
use if_chain::if_chain;
use semver::Version;
use std::{path::Path, rc::Rc, time::Instant};
use toml::Value;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Rev {
    pub version: String,
    pub channel: String,
    pub oid: Oid,
}

// Holds the repository and cached commit data for searching.
pub struct Revs {
    repo: Rc<Repository>,
    quiet: bool,
}

impl Revs {
    pub fn new(quiet: bool) -> Result<Self> {
        let start = Instant::now();
        let repo = clippy_repository(quiet)?;
        if !quiet {
            eprintln!("Revs repository initialization took: {:?}", start.elapsed());
        }
        Ok(Self { repo, quiet })
    }

    // Extracts Rev info from a specific commit by parsing blobs.
    fn get_rev_info_for_commit(&self, commit: &Commit) -> Result<Option<Rev>> {
        let tree = commit.tree().context("Failed to get commit tree")?;
        let oid = commit.id();

        // Try to get version from Cargo.toml
        let version = if_chain! {
            if let Ok(cargo_entry) = tree.get_path(Path::new("Cargo.toml"));
            if let Ok(blob) = self.repo.find_blob(cargo_entry.id());
            if let Ok(content) = std::str::from_utf8(blob.content());
            if let Ok(cargo_toml) = content.parse::<Value>();
            if let Some(package) = cargo_toml.get("package");
            if let Some(version_val) = package.get("version");
            if let Some(version_str) = version_val.as_str();
            then {
                Some(version_str.to_string())
            } else {
                None
            }
        };

        // Return early if version was not found
        let Some(version_str) = version else {
            return Ok(None);
        };

        // Try to get channel from rust-toolchain
        let channel = if_chain! {
            if let Ok(toolchain_entry) = tree.get_path(Path::new("rust-toolchain"));
            if let Ok(blob) = self.repo.find_blob(toolchain_entry.id());
            if let Ok(content) = std::str::from_utf8(blob.content());
            then {
                content.trim().to_string()
            } else {
                return Err(anyhow!("Could not find or parse rust-toolchain file in commit {}", oid))
            }
        };

        Ok(Some(Rev {
            version: version_str,
            channel,
            oid,
        }))
    }

    // Finds the latest Rev by linearly searching backwards from HEAD.
    pub fn find_latest_rev(&self) -> Result<Rev> {
        let start = Instant::now();
        let mut revwalk = self
            .repo
            .revwalk()
            .context("Failed to create revision walker")?;
        revwalk.push_head().context("Failed to push HEAD")?;
        revwalk
            .set_sorting(Sort::TOPOLOGICAL)
            .context("Failed to sort by time")?; // Ensure newest first

        for oid_result in revwalk {
            let oid = oid_result.context("Failed to get commit OID")?;
            let commit = self
                .repo
                .find_commit(oid)
                .context("Failed to find commit")?;
            if let Some(rev_info) = self.get_rev_info_for_commit(&commit)? {
                if !self.quiet {
                    eprintln!(
                        "Found latest version {} in {:?}",
                        rev_info.version,
                        start.elapsed()
                    );
                }
                return Ok(rev_info);
            }
        }

        Err(anyhow!("Could not determine latest `clippy_utils` version"))
    }

    // Finds the Rev for a specific target version using lazy loading and binary search.
    #[allow(clippy::too_many_lines)]
    pub fn find_version(&self, target_version_str: &str) -> Result<Option<Rev>> {
        let start = Instant::now();
        let target_version = Version::parse(target_version_str)
            .with_context(|| format!("Invalid target version string: {target_version_str}"))?;

        let mut commit_data = Vec::<Option<Rev>>::new(); // Stores loaded commits' Rev Option (newest first)
        let mut revwalk = self
            .repo
            .revwalk()
            .context("Failed to create revision walker")?;
        revwalk.push_head().context("Failed to push HEAD")?;
        revwalk
            .set_sorting(Sort::TOPOLOGICAL)
            .context("Failed to sort by time")?;

        let mut limit = 1;
        let mut oldest_commit_rev: Option<Rev> = None;
        let mut head_rev: Option<Rev> = None;

        // 1. Lazy Loading phase
        loop {
            // Use iterator methods to collect the next batch of OIDs
            let current_batch_oids: Vec<Oid> =
                revwalk.by_ref().take(limit).collect::<Result<_, _>>()?;

            let reached_end = current_batch_oids.len() < limit;

            if current_batch_oids.is_empty() && commit_data.is_empty() {
                // If HEAD commit has no parseable version and no history provided.
                return Err(anyhow!("No valid commits found in repository history"));
            }

            for oid in current_batch_oids {
                let commit = self
                    .repo
                    .find_commit(oid)
                    .context("Failed to find commit")?;

                let rev_info = self.get_rev_info_for_commit(&commit)?;

                if head_rev.is_none() {
                    // First commit processed
                    if let Some(ref rev) = rev_info {
                        head_rev = Some(rev.clone());
                        let head_version = Version::parse(&rev.version)?;
                        // Early exit if target is newer than the latest known version
                        if target_version > head_version {
                            if !self.quiet {
                                eprintln!(
                                    "Target version {} is newer than HEAD version {}, search took {:?}",
                                    target_version_str,
                                    rev.version,
                                    start.elapsed()
                                );
                            }
                            return Ok(None);
                        }
                    }
                    // If HEAD has no version, continue loading history.
                }

                // Update the oldest Rev found so far in this loading phase
                if let Some(ref rev) = rev_info {
                    oldest_commit_rev = Some(rev.clone());
                }

                commit_data.push(rev_info);
            }

            if reached_end {
                break;
            }

            // Check if the oldest Rev version found brackets the target version
            if let Some(ref oldest_rev) = oldest_commit_rev {
                let oldest_v = Version::parse(&oldest_rev.version)?;
                if oldest_v < target_version {
                    break;
                }
            }
            // Else: oldest found is still newer than target, or no version found yet. Double limit
            // and continue.

            limit *= 2;
        }

        if !self.quiet {
            eprintln!(
                "Lazy loading phase collected {} commits, took {:?}",
                commit_data.len(),
                start.elapsed()
            );
        }

        // 2. Binary Search phase
        commit_data.reverse();

        let search_start = Instant::now();

        // Use binary_search to find the appropriate Rev
        let result_rev = {
            // First, filter out None values and extract just the versions with their indices
            let versioned_commits: Vec<(usize, &Rev)> = commit_data
                .iter()
                .enumerate()
                .filter_map(|(idx, rev_opt)| rev_opt.as_ref().map(|rev| (idx, rev)))
                .collect();

            if versioned_commits.is_empty() {
                None
            } else {
                // Parse all versions ahead of time to avoid panicking in the binary search
                let mut parsed_versions: Vec<(usize, &Rev, Version)> = Vec::new();

                for (idx, rev) in &versioned_commits {
                    match Version::parse(&rev.version) {
                        Ok(rev_version) => parsed_versions.push((*idx, *rev, rev_version)),
                        Err(_) => {
                            if !self.quiet {
                                eprintln!(
                                    "Warning: Invalid version string in Rev: {}",
                                    rev.version
                                );
                            }
                            // Skip commits with invalid version strings
                        }
                    }
                }

                if parsed_versions.is_empty() {
                    None // No valid versions found
                } else {
                    // Use binary_search_by to find the index of the first Rev
                    let search_result = parsed_versions
                        .binary_search_by(|(_, _, rev_version)| rev_version.cmp(&target_version));

                    // Handle the search result - either exact match or insertion point
                    let index = match search_result {
                        Ok(exact_idx) => exact_idx, // Exact match found
                        Err(insert_idx) => {
                            if insert_idx == 0 {
                                // Target is smaller than all versions, return oldest
                                0
                            } else {
                                // Return the previous version (largest that's < target)
                                insert_idx - 1
                            }
                        }
                    };

                    // Return the Rev at the found index
                    Some(parsed_versions[index].1.clone())
                }
            }
        };

        if !self.quiet {
            eprintln!("Binary search phase took {:?}", search_start.elapsed());
            eprintln!("Total `find_version` took {:?}", start.elapsed());
        }

        Ok(result_rev)
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use super::*;
    use std::sync::LazyLock;

    // OIDs and versions should correspond to commits where the version changed in clippy repo
    static EXAMPLES: LazyLock<Vec<Rev>> = LazyLock::new(|| {
        vec![
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
            // smoelius: 0.1.62 and 0.1.63 omitted (for no particular reason).
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
    fn find_latest() {
        let revs = Revs::new(false).unwrap();
        let latest_rev = revs.find_latest_rev().unwrap();
        println!("Latest Rev found: {:?}", latest_rev);
        assert!(!latest_rev.version.is_empty());
        assert!(!latest_rev.channel.is_empty());
    }

    #[test]
    fn find_specific_versions() {
        let start = Instant::now();
        let revs = Revs::new(false).unwrap();
        let init_duration = start.elapsed();
        println!("Test Initialization took: {:?}", init_duration);

        for example in EXAMPLES.iter() {
            // Example: Use find_version to verify we get the expected version
            println!("Searching for version: {}", example.version);
            let search_start = Instant::now();
            let found_rev_opt = revs.find_version(&example.version).unwrap();
            println!(
                "Search for {} took: {:?}",
                example.version,
                search_start.elapsed()
            );

            assert!(
                found_rev_opt.is_some(),
                "Version {} not found",
                example.version
            );
            let found_rev = found_rev_opt.unwrap();

            // Assert that the found version is > the target version.
            let found_v = Version::parse(&found_rev.version).unwrap();
            let example_v = Version::parse(&example.version).unwrap();
            assert!(
                found_v >= example_v,
                "Found version {} should be >= target {}",
                found_rev.version,
                example.version
            );

            // The binary search finds the first commit where the version requirement
            // is met, which might not be the exact commit in our examples.
            println!("Found Rev for target {}: {:?}", example.version, found_rev);
        }

        println!("Total test duration: {:?}", start.elapsed());
    }

    #[test]
    fn find_non_existent_newer_version() {
        let revs = Revs::new(true).unwrap();
        let future_version = "999.0.0"; // Assumed to be newer than anything in clippy repo
        let result = revs.find_version(future_version).unwrap();
        assert!(
            result.is_none(),
            "Should not find a version newer than HEAD"
        );
    }

    #[test]
    fn find_version_older_than_history() {
        // Test that searching for a version older than any known version
        // returns the oldest known version.
        let revs = Revs::new(true).unwrap();
        let ancient_version = "0.0.1"; // Version older than any in EXAMPLES

        // Find the expected oldest Rev from the static EXAMPLES list
        let expected_oldest_rev = EXAMPLES
            .iter()
            .min_by(|a, b| {
                Version::parse(&a.version)
                    .unwrap()
                    .cmp(&Version::parse(&b.version).unwrap())
            })
            .unwrap();

        let result = revs.find_version(ancient_version).unwrap();

        assert!(
            result.is_some(),
            "Searching for ancient version {} should return the oldest known version, but got None",
            ancient_version
        );

        let found_rev = result.unwrap();

        let found_v = Version::parse(&found_rev.version).unwrap();
        let expected_oldest_v = Version::parse(&expected_oldest_rev.version).unwrap();

        assert!(
            found_v >= expected_oldest_v,
            "Found version {} for ancient target should be >= oldest example version {}",
            found_rev.version,
            expected_oldest_rev.version
        );

        println!(
            "Search for ancient version {} found Rev: {:?}",
            ancient_version, found_rev
        );
    }
}
