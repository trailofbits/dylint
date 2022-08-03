#[cfg(test)]
mod test {
    use dylint_internal::{examples::iter, rustup::SanitizeEnvironment};

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
}
