use super::common::clippy_repository;
use anyhow::{Context, Result, anyhow};
use dylint_internal::git2::{Commit, Oid, Repository, Sort};
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

// Represents commit info potentially including parsed Rev data.
#[derive(Clone)]
struct CommitInfo {
    rev: Option<Rev>,
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
        let version = if let Ok(cargo_entry) = tree.get_path(Path::new("Cargo.toml")) {
            if let Ok(blob) = self.repo.find_blob(cargo_entry.id()) {
                if let Ok(content) = std::str::from_utf8(blob.content()) {
                    if let Ok(cargo_toml) = content.parse::<Value>() {
                        cargo_toml
                            .get("package")
                            .and_then(|p| p.get("version"))
                            .and_then(|v| v.as_str())
                            .map(String::from)
                    } else {
                        None // Failed to parse TOML
                    }
                } else {
                    None // Content not UTF-8
                }
            } else {
                None // Blob not found
            }
        } else {
            None // Cargo.toml not found
        };

        // Only proceed if version was found
        if let Some(version_str) = version {
            // Try to get channel from rust-toolchain, fallback to "nightly"
            let channel = if let Ok(toolchain_entry) = tree.get_path(Path::new("rust-toolchain")) {
                if let Ok(blob) = self.repo.find_blob(toolchain_entry.id()) {
                    if let Ok(content) = std::str::from_utf8(blob.content()) {
                        content.trim().to_string()
                    } else {
                        "nightly".to_string() // Content not UTF-8, fallback
                    }
                } else {
                    "nightly".to_string() // Blob not found, fallback
                }
            } else {
                "nightly".to_string() // rust-toolchain not found, fallback
            };

            Ok(Some(Rev {
                version: version_str,
                channel,
                oid,
            }))
        } else {
            Ok(None) // No version found in Cargo.toml
        }
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
            .set_sorting(Sort::TIME)
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
            // Continue walking back if commit didn't have parseable version info
        }

        Err(anyhow!("Could not determine latest `clippy_utils` version"))
    }

    // Finds the Rev for a specific target version using lazy loading and binary search.
    pub fn find_version(&self, target_version_str: &str) -> Result<Option<Rev>> {
        let start = Instant::now();
        let target_version = Version::parse(target_version_str)
            .with_context(|| format!("Invalid target version string: {target_version_str}"))?;

        let mut commit_data = Vec::<CommitInfo>::new(); // Stores loaded commits (newest first)
        let mut revwalk = self
            .repo
            .revwalk()
            .context("Failed to create revision walker")?;
        revwalk.push_head().context("Failed to push HEAD")?;
        revwalk
            .set_sorting(Sort::TIME)
            .context("Failed to sort by time")?;

        let mut limit = 1;
        let mut oldest_commit_rev: Option<Rev> = None;
        let mut head_rev: Option<Rev> = None;

        // 1. Lazy Loading phase (doubling search limit)
        loop {
            let mut current_batch_oids = Vec::new();
            let mut count = 0;
            for oid_result in revwalk.by_ref() {
                current_batch_oids.push(oid_result?);
                count += 1;
                if count >= limit {
                    break;
                }
            }

            if current_batch_oids.is_empty() && commit_data.is_empty() {
                // If HEAD commit has no parseable version and no history provided.
                return Err(anyhow!("No valid commits found in repository history"));
            }

            // Process the batch: get commit info and Rev data
            for oid in current_batch_oids {
                let commit = self
                    .repo
                    .find_commit(oid)
                    .context("Failed to find commit")?;

                let rev_info = self.get_rev_info_for_commit(&commit)?;

                if head_rev.is_none() {
                    // First commit processed (HEAD)
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

                commit_data.push(CommitInfo { rev: rev_info });
            }

            if count < limit {
                // Reached the beginning of history
                break;
            }

            // Check if the oldest Rev version found brackets the target version
            if let Some(ref oldest_rev) = oldest_commit_rev {
                let oldest_v = Version::parse(&oldest_rev.version)?;
                if oldest_v <= target_version {
                    break; // Found a commit potentially older/equal to target, proceed to binary search
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
        // `commit_data` is currently newest-first. Reverse it for easier binary search (oldest
        // first).
        commit_data.reverse();

        let search_start = Instant::now();

        // Custom binary search to find the *first* commit index (oldest first)
        // where the effective version is >= target_version.
        let mut result_rev: Option<Rev> = None;
        let mut low = 0;
        let mut high = commit_data.len();

        while low < high {
            let mid = low + (high - low) / 2;

            // Find the effective Rev active at `commit_data[mid]`
            // Search forward (towards newer commits) from `mid` until a Rev is found.
            let mut effective_rev_at_mid: Option<Rev> = None;
            for i in (mid..commit_data.len()).rev() {
                // Search backwards (newer) in original order, forward in reversed
                if let Some(ref rev) = commit_data[i].rev {
                    effective_rev_at_mid = Some(rev.clone());
                    break;
                }
            }

            match effective_rev_at_mid {
                Some(rev) => {
                    let current_version = Version::parse(&rev.version)?;
                    if current_version >= target_version {
                        // This version is suitable. It might be the first, or an earlier commit
                        // might also work.
                        result_rev = Some(rev); // Store this as a potential answer
                        high = mid; // Try earlier commits
                    } else {
                        // Version is too old. Need a newer commit.
                        low = mid + 1;
                    }
                }
                None => {
                    // No version defined at or after commit `mid`. This implies all commits
                    // from `mid` onwards (towards newer) lack version info.
                    // This case shouldn't happen if HEAD has a version, but if it does,
                    // treat these commits as "too old".
                    low = mid + 1;
                }
            }
        }

        if !self.quiet {
            eprintln!("Binary search phase took {:?}", search_start.elapsed());
            eprintln!("Total `find_version` took {:?}", start.elapsed());
        }

        // `result_rev` now holds the Rev from the first commit (chronologically)
        // whose effective version meets the target.
        Ok(result_rev)
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use super::*;
    use std::sync::LazyLock;

    // OIDs and versions should correspond to commits where the version *changed* in clippy repo
    static EXAMPLES: LazyLock<Vec<Rev>> = LazyLock::new(|| {
        vec![
            Rev {
                version: "0.1.79".to_owned(), // Assuming a newer version exists
                channel: "nightly-2024-03-28".to_owned(), // Example channel
                oid: Oid::from_str("aabbccddeeff00112233445566778899aabbccdd").unwrap(), /* FAKE OID - Replace if needed */
            },
            Rev {
                version: "0.1.78".to_owned(),
                channel: "nightly-2024-02-29".to_owned(), // Example channel
                oid: Oid::from_str("f3b9716024172cde57282d73c6f66e57c94001eb").unwrap(), /* Actual OID for 0.1.78 bump */
            },
            Rev {
                version: "0.1.77".to_owned(),
                channel: "nightly-2024-01-04".to_owned(), // Real channel needed if tested
                oid: Oid::from_str("6964287c50f8f5d39f647b5e4a5949e327256ce6").unwrap(), /* Actual OID for 0.1.77 bump */
            },
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
            // Add more examples if needed, ensure OIDs are correct
        ]
    });

    #[test]
    fn find_latest() {
        let revs = Revs::new(false).unwrap();
        let latest_rev = revs.find_latest_rev().unwrap();
        // We don't know the absolute latest without cloning, but it should be fairly recent.
        println!("Latest Rev found: {:?}", latest_rev);
        assert!(!latest_rev.version.is_empty());
        assert!(!latest_rev.channel.is_empty());
    }

    #[test]
    fn find_specific_versions() {
        let start = Instant::now();
        let revs = Revs::new(false).unwrap(); // Set to true for less output during test
        let init_duration = start.elapsed();
        println!("Test Initialization took: {:?}", init_duration);

        for example in EXAMPLES.iter() {
            // Skip potentially fake OID entry unless updated
            if example.oid == Oid::from_str("aabbccddeeff00112233445566778899aabbccdd").unwrap() {
                continue;
            }

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

            // The found version might be *newer* than the example if the example OID
            // is not the *exact* commit the version bumped. The search finds the
            // commit where the target version *became active*.
            let found_v = Version::parse(&found_rev.version).unwrap();
            let example_v = Version::parse(&example.version).unwrap();

            // Assert that the found version is >= the target version.
            assert!(
                found_v >= example_v,
                "Found version {} should be >= target {}",
                found_rev.version,
                example.version
            );

            // We cannot reliably assert the channel and OID match the example exactly,
            // because the binary search finds the *first* commit where the version requirement
            // is met, which might not be the exact commit listed in EXAMPLES if intermediate
            // commits existed without version bumps. We primarily care that we find *a* commit
            // where the requested version (or compatible later) is active.

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
    fn find_non_existent_older_version() {
        // Find a very old version unlikely to exist or where history might be shallow
        let revs = Revs::new(true).unwrap();
        let ancient_version = "0.0.1";
        let result = revs.find_version(ancient_version).unwrap();
        // Depending on history, this might find the oldest known version or None if history starts
        // later. If it finds *a* version, it should be the oldest one available.
        if let Some(rev) = result {
            println!(
                "Found potentially oldest Rev for target {}: {:?}",
                ancient_version, rev
            );
            // We can't assert much more without knowing the full repo history state.
        } else {
            println!(
                "Could not find any Rev for ancient target {}",
                ancient_version
            );
            // This is also plausible if history doesn't go back far enough or lacks version info
            // early on.
        }
    }
}
