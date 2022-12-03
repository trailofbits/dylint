#[cfg(test)]
mod test {
    use cargo_metadata::MetadataCommand;
    use dylint_internal::{
        clippy_utils::toolchain_channel, examples::iter, rustup::SanitizeEnvironment,
    };
    use std::{ffi::OsStr, fs::read_to_string};

    #[test]
    fn examples() {
        for path in iter().unwrap() {
            let path = path.unwrap();
            let file_name = path.file_name().unwrap();
            dylint_internal::cargo::test(
                &format!("example `{}`", file_name.to_string_lossy()),
                false,
            )
            .sanitize_environment()
            .current_dir(path)
            .success()
            .unwrap();
        }
    }

    #[test]
    fn examples_have_same_version_as_workspace() {
        for path in iter().unwrap() {
            let path = path.unwrap();
            let metadata = MetadataCommand::new()
                .current_dir(path)
                .no_deps()
                .exec()
                .unwrap();
            let package = metadata.root_package().unwrap();
            assert_eq!(package.version.to_string(), env!("CARGO_PKG_VERSION"));
        }
    }

    #[test]
    fn examples_have_identical_cargo_configs() {
        let mut prev = None;
        for path in iter().unwrap() {
            let path = path.unwrap();
            if path.file_name() == Some(OsStr::new("straggler")) {
                continue;
            }
            let config_toml = path.join(".cargo/config.toml");
            let curr = read_to_string(config_toml).unwrap();
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
        for path in iter().unwrap() {
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
}
