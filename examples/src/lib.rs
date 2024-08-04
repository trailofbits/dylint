#[cfg(all(not(coverage), test))]
mod test {
    use cargo_metadata::MetadataCommand;
    use dylint_internal::{
        clippy_utils::toolchain_channel, examples::iter, rustup::SanitizeEnvironment, CommandExt,
    };
    use std::{ffi::OsStr, fs::read_to_string};
    use toml_edit::{DocumentMut, Item, Value};
    use walkdir::WalkDir;

    #[test]
    fn examples() {
        for path in iter(false).unwrap() {
            let path = path.unwrap();
            let file_name = path.file_name().unwrap();
            // smoelius: Pass `--lib --tests` to `cargo test` to avoid the potential filename
            // collision associated with building the examples.
            dylint_internal::cargo::test(&format!("example `{}`", file_name.to_string_lossy()))
                .build()
                .sanitize_environment()
                .current_dir(path)
                .args(["--lib", "--tests"])
                .success()
                .unwrap();
        }
    }

    #[test]
    fn examples_have_same_version_as_workspace() {
        for path in iter(false).unwrap() {
            let path = path.unwrap();
            if path.file_name() == Some(OsStr::new("restriction")) {
                continue;
            }
            let metadata = MetadataCommand::new()
                .current_dir(&path)
                .no_deps()
                .exec()
                .unwrap();
            let package = dylint_internal::cargo::package_with_root(&metadata, &path).unwrap();
            assert_eq!(env!("CARGO_PKG_VERSION"), package.version.to_string());
        }
    }

    #[test]
    fn examples_have_equivalent_cargo_configs() {
        let mut prev = None;
        for path in iter(true).unwrap() {
            let path = path.unwrap();
            if path.file_name() == Some(OsStr::new("straggler")) {
                continue;
            }
            let config_toml = path.join(".cargo/config.toml");
            let contents = read_to_string(config_toml).unwrap();
            let mut document = contents.parse::<DocumentMut>().unwrap();
            // smoelius: Hack. `build.target-dir` is expected to be a relative path. Replace it with
            // an absolute one. However, the directory might not exist when this test is run. So use
            // `cargo_util::paths::normalize_path` rather than `Path::canonicalize`.
            document
                .as_table_mut()
                .get_mut("build")
                .and_then(Item::as_table_mut)
                .and_then(|table| table.get_mut("target-dir"))
                .and_then(Item::as_value_mut)
                .and_then(|value| {
                    let target_dir = value.as_str()?;
                    *value = cargo_util::paths::normalize_path(&path.join(target_dir))
                        .to_string_lossy()
                        .as_ref()
                        .into();
                    Some(())
                })
                .unwrap();
            let curr = document.to_string();
            if let Some(prev) = &prev {
                assert_eq!(*prev, curr);
            } else {
                prev = Some(curr);
            }
        }
    }

    #[test]
    fn examples_use_same_toolchain_channel() {
        let mut prev = None;
        for path in iter(true).unwrap() {
            let path = path.unwrap();
            if path.file_name() == Some(OsStr::new("marker")) {
                continue;
            }
            if path.file_name() == Some(OsStr::new("straggler")) {
                continue;
            }
            let curr = toolchain_channel(&path).unwrap();
            if let Some(prev) = &prev {
                assert_eq!(*prev, curr);
            } else {
                prev = Some(curr);
            }
        }
    }

    #[test]
    fn examples_do_not_require_rust_src() {
        for path in iter(true).unwrap() {
            let path = path.unwrap();

            let contents = read_to_string(path.join("rust-toolchain")).unwrap();
            let document = contents.parse::<DocumentMut>().unwrap();
            let array = document
                .as_table()
                .get("toolchain")
                .and_then(Item::as_table)
                .and_then(|table| table.get("components"))
                .and_then(Item::as_array)
                .unwrap();
            let components = array
                .iter()
                .map(Value::as_str)
                .collect::<Option<Vec<_>>>()
                .unwrap();

            assert!(!components.contains(&"rust-src"));
        }
    }

    #[test]
    fn examples_do_not_have_gitignore() {
        for entry in WalkDir::new(".") {
            let entry = entry.unwrap();
            let path = entry.path();
            assert_ne!(
                path.file_name(),
                Some(OsStr::new(".gitignore")),
                "failed for {path:?}"
            );
        }
    }
}
