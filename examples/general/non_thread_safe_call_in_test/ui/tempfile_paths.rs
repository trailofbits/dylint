fn main() {}

#[cfg(test)]
mod tests {
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn positive_paths() {
        let named = NamedTempFile::new().unwrap();
        let named_from_parts = NamedTempFile::from_parts(
            tempfile::tempfile().unwrap(),
            tempfile::TempPath::try_from_path("shared-named").unwrap(),
        );
        let temp_path = tempfile::TempPath::try_from_path("shared-temp-path").unwrap();

        // The destination is shared, even though the source is temporary.
        std::fs::copy(named.path(), "shared-copy").unwrap();

        // `NamedTempFile` can wrap an arbitrary path.
        std::fs::write(named_from_parts.path(), "value").unwrap();

        // `NamedTempFile` implements `AsRef<Path>`.
        std::fs::write(&named_from_parts, "value").unwrap();

        // `TempPath` can wrap an arbitrary path.
        std::fs::write(&temp_path, "value").unwrap();

        // A path not from a temporary file or directory.
        std::fs::write("shared", "value").unwrap();
    }

    #[test]
    fn negative_tempdir_paths() {
        let tempdir = tempfile::tempdir().unwrap();

        // The shared source is only read; the destination is temporary.
        std::fs::copy("shared", tempdir.path().join("copy")).unwrap();

        // The path is directly under `tempdir`.
        std::fs::write(tempdir.path().join("direct"), "value").unwrap();

        // Both immutable bindings lead back to a path under `tempdir`.
        let path = tempdir.path().join("alias");
        let alias = path;
        std::fs::write(alias, "value").unwrap();

        // `TempDir` implements `AsRef<Path>`.
        std::fs::create_dir_all(&tempdir).unwrap();

        // The helper receives a temporary directory.
        write_inside_tempdir(&tempdir);
    }

    fn write_inside_tempdir(tempdir: &TempDir) {
        // The helper's path comes from its temporary directory argument.
        std::fs::write(tempdir.path().join("helper"), "value").unwrap();
    }
}
