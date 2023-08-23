use crate::cargo::{current_metadata, package};
use anyhow::{anyhow, Context, Result};
use rust_embed::RustEmbed;
use std::{
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::Path,
};

#[derive(RustEmbed)]
#[folder = "template"]
#[exclude = "Cargo.lock"]
#[exclude = "target/*"]
struct Template;

pub fn new_template(to: &Path) -> Result<()> {
    for path in Template::iter() {
        let embedded_file = Template::get(&path)
            .ok_or_else(|| anyhow!("Could not get embedded file `{}`", path))?;
        let to_path = to.join(path.trim_end_matches('~'));
        let parent = to_path
            .parent()
            .ok_or_else(|| anyhow!("Could not get parent directory"))?;
        create_dir_all(parent).with_context(|| {
            format!("`create_dir_all` failed for `{}`", parent.to_string_lossy())
        })?;
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&to_path)
            .with_context(|| format!("Could not open `{}`", to_path.to_string_lossy()))?;
        file.write_all(&embedded_file.data)
            .with_context(|| format!("Could not write to {to_path:?}"))?;
    }

    Ok(())
}

// smoelius: If a package is checked out in the current directory, this must be dealt with:
// error: current package believes it's in a workspace when it's not
pub fn isolate(path: &Path) -> Result<()> {
    let manifest = path.join("Cargo.toml");
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(&manifest)
        .with_context(|| format!("Could not open `{}`", manifest.to_string_lossy()))?;

    writeln!(file)
        .and_then(|_| writeln!(file, "[workspace]"))
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
        .write(true)
        .append(true)
        .open(&manifest)
        .with_context(|| format!("Could not open `{}`", manifest.to_string_lossy()))?;

    // smoelius: `use_local_packages` broke when `dylint_linting` was removed from the workspace.
    // For now, add `dylint_linting` manually.
    writeln!(file)
        .and_then(|_| writeln!(file, "[patch.crates-io]"))
        .and_then(|_| {
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
                .all(|target| target.kind.iter().all(|kind| kind != "lib"))
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
    use std::fs::read_to_string;
    use toml_edit::{Document, Item};

    #[cfg_attr(
        dylint_lib = "assert_eq_arg_misordering",
        allow(assert_eq_arg_misordering)
    )]
    #[test]
    fn template_includes_only_whitelisted_paths() {
        const PATHS: [&str; 8] = [
            ".cargo/config.toml",
            ".gitignore",
            "Cargo.toml~",
            "README.md",
            "rust-toolchain",
            "src/lib.rs",
            "ui/main.rs",
            "ui/main.stderr",
        ];

        let mut paths_sorted = PATHS.to_vec();
        paths_sorted.sort_unstable();
        assert_eq!(paths_sorted, PATHS);

        let paths = Template::iter()
            .filter(|path| PATHS.binary_search(&&**path).is_err())
            .collect::<Vec<_>>();

        assert!(paths.is_empty(), "found {paths:#?}");
    }

    #[test]
    fn template_has_initial_version() {
        let contents =
            read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("template/Cargo.toml~"))
                .unwrap();
        let document = contents.parse::<Document>().unwrap();
        let version = document
            .as_table()
            .get("package")
            .and_then(Item::as_table)
            .and_then(|table| table.get("version"))
            .and_then(Item::as_str)
            .unwrap();
        assert_eq!("0.1.0", version);
    }
}
