#[cfg(test)]
mod test {
    use dylint_internal::{examples::iter, rustup::SanitizeEnvironment};
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
    fn examples_have_identical_cargo_configs() {
        let mut prev = None;
        for path in iter().unwrap() {
            let path = path.unwrap();
            if path.file_name() == Some(OsStr::new("straggler")) {
                continue;
            }
            let config_toml = path.join(".cargo").join("config.toml");
            let curr = read_to_string(config_toml).unwrap();
            if let Some(prev) = &prev {
                assert_eq!(*prev, curr);
            } else {
                prev = Some(curr);
            }
        }
    }
}
