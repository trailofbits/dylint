#[cfg(all(not(coverage), test))]
mod test {
    use cargo_metadata::MetadataCommand;
    use dylint_internal::{CommandExt, clippy_utils::toolchain_channel, examples::iter};
    use regex::Regex;
    use std::{
        collections::BTreeSet,
        ffi::OsStr,
        fs::{read_dir, read_to_string},
        path::Path,
        process::Command,
    };
    use toml_edit::{DocumentMut, Item};
    use walkdir::WalkDir;

    #[test]
    fn examples() {
        // smoelius: Pass `--lib --tests` to `cargo test` to avoid the potential filename
        // collision associated with building the examples.
        nested_workspace::test().args(["--lib", "--tests"]).unwrap();
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
    fn examples_use_edition_2024() {
        for result in iter(false).unwrap() {
            let manifest_dir = result.unwrap();
            let manifest_path = manifest_dir.join("Cargo.toml");
            let contents = read_to_string(&manifest_path).unwrap();
            let table = toml::from_str::<toml::Table>(&contents).unwrap();
            let Some(package) = table.get("package").and_then(|value| value.as_table()) else {
                continue;
            };
            let edition = package.get("edition").and_then(toml::Value::as_str);
            assert_eq!(
                Some("2024"),
                edition,
                "failed for `{}`",
                manifest_path.display()
            );
        }
    }

    #[test]
    fn examples_use_same_toolchain_channel() {
        let mut prev = None;
        for path in iter(true).unwrap() {
            let path = path.unwrap();
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
            let table = toml::from_str::<toml::Table>(&contents).unwrap();
            let array = table
                .get("toolchain")
                .and_then(toml::Value::as_table)
                .and_then(|toolchain| toolchain.get("components"))
                .and_then(toml::Value::as_array)
                .unwrap();

            let components = array
                .iter()
                .map(toml::Value::as_str)
                .collect::<Option<Vec<_>>>()
                .unwrap();

            assert!(!components.contains(&"rust-src"));
        }
    }

    #[test]
    fn examples_do_not_contain_forbidden_paths() {
        let forbidden_files_general = [".gitignore"];
        let forbidden_files_specific = [".cargo/config.toml", "rust-toolchain"];
        let allowed_dirs = ["experimental", "testing"];
        let root_dirs_with_exceptions = ["general", "supplementary", "restriction"];

        for entry in WalkDir::new(".").into_iter().flatten() {
            let file_name = entry.file_name().to_str().unwrap();
            let path = entry.path();

            if forbidden_files_general.contains(&file_name) {
                assert!(
                    !forbidden_files_general.contains(&file_name),
                    "Forbidden file `.gitignore` found in examples directory: {}",
                    path.display()
                );
            }

            if forbidden_files_specific.contains(&file_name) {
                let is_in_allowed_directory = allowed_dirs
                    .iter()
                    .any(|&allowed| path.starts_with(allowed));

                let is_in_root_of_exception_dirs =
                    root_dirs_with_exceptions.iter().any(|&exception| {
                        path.starts_with(exception) && path.components().count() == 2
                    });

                assert!(
                    !(is_in_allowed_directory || is_in_root_of_exception_dirs),
                    "Forbidden file {} found in non-allowed directory: {}",
                    file_name,
                    path.display()
                );
            }
        }
    }

    #[test]
    fn check_examples_formatting() {
        let mut failed_files = Vec::new();

        for entry in WalkDir::new(".")
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension() == Some(OsStr::new("rs")))
        {
            let path = entry.path();
            let output = Command::new("rustfmt")
                .args([
                    "+nightly",
                    "--check",
                    "--edition=2024",
                    path.to_str().unwrap(),
                ])
                .logged_output(false)
                .unwrap_or_else(|error| panic!("Failed to execute rustfmt: {error}"));

            if !output.status.success() {
                failed_files.push(path.to_path_buf());
                eprintln!(
                    "rustfmt check failed for: {}\nstdout:\n```\n{}\n```\nstderr:\n```\n{}\n```",
                    path.display(),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        assert!(
            failed_files.is_empty(),
            "rustfmt check failed for the following files:\n{}",
            failed_files
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn verify_registered_lints() {
        const CATEGORIES: &[&str] = &["general", "supplementary"];
        let register_lints_re =
            Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)::register_lints\s*\(").unwrap();

        for category in CATEGORIES {
            let category_path = Path::new(category);
            let lib_rs_path = category_path.join("src/lib.rs");
            let file_contents = read_to_string(&lib_rs_path)
                .unwrap_or_else(|e| panic!("Failed to read {}: {}", lib_rs_path.display(), e));

            let actual_lints: BTreeSet<_> = register_lints_re
                .captures_iter(&file_contents)
                .map(|cap| cap.get(1).unwrap().as_str().to_string())
                .collect();

            let expected_lints: BTreeSet<_> = read_dir(category_path)
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to read directory {}: {}",
                        category_path.display(),
                        e
                    )
                })
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    if path.is_dir() {
                        let file_name = entry.file_name();
                        let dir_name = file_name.to_str()?;
                        if dir_name != "src" && !dir_name.starts_with('.') {
                            return Some(dir_name.to_string());
                        }
                    }
                    None
                })
                .collect();

            let missing: Vec<_> = expected_lints.difference(&actual_lints).collect();

            assert!(
                missing.is_empty(),
                "Mismatch in {}\n\nMissing registered lints: {:?}\n\nExpected: {:?}\nActual: {:?}",
                category_path.display(),
                missing,
                expected_lints,
                actual_lints
            );
        }
    }
}
