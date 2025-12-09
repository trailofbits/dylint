// smoelius: Since the update to `rust_embed` 8.3.0, `unnecessary_conversion_for_trait` started
// firing on `struct Template`. Requiring `!expr.span.from_expansion()` in
// `unnecessary_conversion_for_trait` causes one of its tests to fail. So allow the lint for now.
// smoelius: `abs_home_path` now fires as well.
#![cfg_attr(dylint_lib = "general", allow(abs_home_path))]
#![cfg_attr(dylint_lib = "supplementary", allow(unnecessary_conversion_for_trait))]

use crate::cargo::{current_metadata, package};
use anyhow::{Context, Result, anyhow};
use cargo_metadata::TargetKind;
use std::{fs::OpenOptions, io::Write, path::Path};
use tar::Archive;

const TEMPLATE_TAR: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/template.tar"));

pub fn new_template(to: &Path) -> Result<()> {
    Archive::new(TEMPLATE_TAR)
        .unpack(to)
        .with_context(|| "Could not unpack archive")?;

    Ok(())
}

// smoelius: If a package is checked out in the current directory, this must be dealt with:
// error: current package believes it's in a workspace when it's not
pub fn isolate(path: &Path) -> Result<()> {
    let manifest = path.join("Cargo.toml");
    let mut file = OpenOptions::new()
        .append(true)
        .open(&manifest)
        .with_context(|| format!("Could not open `{}`", manifest.to_string_lossy()))?;

    writeln!(file)
        .and_then(|()| writeln!(file, "[workspace]"))
        .with_context(|| format!("Could not write to `{}`", manifest.to_string_lossy()))?;

    Ok(())
}

// smoelius: If you clone, say, `dylint-template` and run `cargo test` on it, it will obtain Dylint
// packages from `crates.io`. But for the tests in this repository, you often want it to use the
// packages in this repository. The function `use_local_packages` patches a workspace's `Cargo.toml`
// file to do so.
pub fn use_local_packages(path: &Path) -> Result<()> {
    let metadata = current_metadata()?;

    let manifest = path.join("Cargo.toml");

    let mut file = OpenOptions::new()
        .append(true)
        .open(&manifest)
        .with_context(|| format!("Could not open `{}`", manifest.to_string_lossy()))?;

    // smoelius: `use_local_packages` broke when `dylint_linting` was removed from the workspace.
    // For now, add `dylint_linting` manually.
    writeln!(file)
        .and_then(|()| writeln!(file, "[patch.crates-io]"))
        .and_then(|()| {
            writeln!(
                file,
                r#"dylint_linting = {{ path = "{}" }}"#,
                metadata
                    .workspace_root
                    .join("utils/linting")
                    .to_string()
                    .replace('\\', "\\\\")
            )
        })
        .with_context(|| format!("Could not write to `{}`", manifest.to_string_lossy()))?;

    for package_id in &metadata.workspace_members {
        let package = package(&metadata, package_id)?;
        if package.publish == Some(vec![])
            || package
                .targets
                .iter()
                .all(|target| target.kind.iter().all(|kind| kind != &TargetKind::Lib))
        {
            continue;
        }
        let path = package
            .manifest_path
            .parent()
            .ok_or_else(|| anyhow!("Could not get parent directory"))?;
        writeln!(
            file,
            r#"{} = {{ path = "{}" }}"#,
            package.name,
            path.to_string().replace('\\', "\\\\")
        )
        .with_context(|| format!("Could not write to `{}`", manifest.to_string_lossy()))?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::env;
    use std::fs::{copy, read, read_to_string};
    use tempfile::TempDir;

    #[cfg_attr(
        dylint_lib = "assert_eq_arg_misordering",
        allow(assert_eq_arg_misordering)
    )]
    #[test]
    fn template_includes_only_whitelisted_paths() {
        const PATHS: [&str; 8] = [
            ".cargo/config.toml",
            ".gitignore",
            "Cargo.toml",
            "README.md",
            "rust-toolchain",
            "src/lib.rs",
            "ui/main.rs",
            "ui/main.stderr",
        ];

        let mut paths_sorted = PATHS.to_vec();
        paths_sorted.sort_unstable();
        assert_eq!(paths_sorted, PATHS);

        let mut archive = Archive::new(TEMPLATE_TAR);
        let entries = archive.entries().unwrap();
        let mut paths = Vec::new();
        for result in entries {
            let entry = result.unwrap();
            let path = entry.path().unwrap();
            let path_str = path.to_str().map(ToOwned::to_owned).unwrap();
            if PATHS.binary_search(&path_str.as_str()).is_err() {
                paths.push(path_str);
            }
        }

        assert!(paths.is_empty(), "found {paths:#?}");
    }

    #[test]
    fn template_has_initial_version() {
        let contents = read_to_string("template/Cargo.toml").unwrap();
        let document = contents.parse::<toml::Table>().unwrap();
        let version = document
            .get("package")
            .and_then(toml::Value::as_table)
            .and_then(|table| table.get("version"))
            .and_then(toml::Value::as_str)
            .unwrap();
        assert_eq!("0.1.0", version);
    }

    #[cfg_attr(dylint_lib = "general", allow(non_thread_safe_call_in_test))]
    #[test]
    fn check_template_tar() {
        let dir = build_template_tar();
        let path_buf = dir.path().join("template.tar");
        if env::enabled("BLESS") {
            copy(
                path_buf,
                concat!(env!("CARGO_MANIFEST_DIR"), "/template.tar"),
            )
            .unwrap();
        } else {
            let contents = read(dir.path().join("template.tar")).unwrap();
            assert_eq!(TEMPLATE_TAR, contents);
        }
    }

    fn build_template_tar() -> TempDir {
        use std::{fs::File, path::Path};
        use tar::{Builder, HeaderMode};
        use walkdir::WalkDir;

        let tempdir = TempDir::new().unwrap();
        let path_buf = tempdir.path().join("template.tar");
        let file = File::create(path_buf).unwrap();
        let mut archive = Builder::new(file);
        archive.mode(HeaderMode::Deterministic);
        let root = Path::new("template");
        for result in WalkDir::new(root)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|entry| {
                let filename = entry.file_name();
                filename != "Cargo.lock" && filename != "target"
            })
        {
            let entry = result.unwrap();
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let mut file = File::open(path).unwrap();
            let path_stripped = path.strip_prefix(root).unwrap();
            archive.append_file(path_stripped, &mut file).unwrap();
        }
        tempdir
    }
}
