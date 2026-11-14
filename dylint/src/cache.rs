use crate::flock;
use anyhow::{Context, Result, anyhow, ensure};
use dylint_internal::env;
use std::{
    env::{home_dir, var_os},
    fs::{File, create_dir_all, read_dir, rename},
    path::{Path, PathBuf},
};
use tempfile::tempdir_in;

pub const DRIVER_PACKAGE: &str = "dylint-driver";

pub fn directory(name: &str) -> Result<PathBuf> {
    let root = if let Some(directory) = var_os(env::DYLINT_CACHE_DIR) {
        let directory = PathBuf::from(directory);
        ensure!(
            directory.is_absolute(),
            "`DYLINT_CACHE_DIR` must be an absolute path, got `{}`",
            directory.display()
        );
        directory
    } else {
        let home = home_dir().ok_or_else(|| anyhow!("Could not find HOME directory"))?;
        home.join(".dylint/cache")
    };
    Ok(root.join(name))
}

#[cfg(test)]
fn migrate(source: &Path, destination: &Path) -> Result<()> {
    migrate_with(source, destination, |_| Ok(true))
}

#[cfg(any(feature = "__cargo_cli", test))]
pub fn migrate_library(
    source: &Path,
    destination: &Path,
    name: &str,
    toolchain: &str,
) -> Result<()> {
    use dylint_internal::{library_filename_with_toolchain, library_plain_filename};
    use std::{fs::remove_file, io::ErrorKind};

    if !source.join("release").is_dir() {
        return Ok(());
    }
    migrate_with(source, destination, |staging| {
        // dblock: Unpartitioned outputs have unknown provenance; Cargo must validate them.
        for filename in [
            library_plain_filename(name),
            library_filename_with_toolchain(name, toolchain),
        ] {
            if let Err(error) = remove_file(staging.join("release").join(filename))
                && error.kind() != ErrorKind::NotFound
            {
                return Err(error.into());
            }
        }
        Ok(true)
    })
}

pub(crate) fn migrate_with(
    source: &Path,
    destination: &Path,
    prepare: impl FnOnce(&Path) -> Result<bool>,
) -> Result<()> {
    if destination
        .try_exists()
        .with_context(|| format!("Could not inspect cache `{}`", destination.display()))?
        || !source.is_dir()
    {
        return Ok(());
    }

    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("Could not find cache parent directory"))?;
    create_dir_all(parent)
        .with_context(|| format!("Could not create cache directory `{}`", parent.display()))?;
    let _destination_lock = flock::lock_path(parent)
        .with_context(|| format!("Could not lock cache directory `{}`", parent.display()))?;
    if destination
        .try_exists()
        .with_context(|| format!("Could not inspect cache `{}`", destination.display()))?
    {
        return Ok(());
    }

    // dblock: Coordinate with legacy Cargo builds or driver installation while copying.
    let cargo_lock = source.join("release/.cargo-lock");
    let _source_lock = if cargo_lock.is_file() {
        flock::lock_file(&cargo_lock)
            .with_context(|| format!("Could not lock legacy cache `{}`", cargo_lock.display()))?
    } else {
        flock::lock_path(source)
            .with_context(|| format!("Could not lock legacy cache `{}`", source.display()))?
    };
    let staging = tempdir_in(parent).with_context(|| {
        format!(
            "Could not create staging directory in `{}`",
            parent.display()
        )
    })?;
    let mut options = fs_extra::dir::CopyOptions::new();
    options.content_only = true;
    fs_extra::dir::copy(source, &staging, &options).with_context(|| {
        format!(
            "Could not migrate cache from `{}` to `{}`",
            source.display(),
            destination.display()
        )
    })?;
    preserve_modified_times(source, staging.path())?;
    if !prepare(staging.path())? {
        return Ok(());
    }
    if let Err(error) = rename(&staging, destination)
        && !destination.is_dir()
    {
        return Err(error)
            .with_context(|| format!("Could not install cache at `{}`", destination.display()));
    }
    Ok(())
}

fn preserve_modified_times(source: &Path, destination: &Path) -> Result<()> {
    let entries = read_dir(source)
        .with_context(|| format!("Could not read legacy cache `{}`", source.display()))?;
    for entry in entries {
        let entry =
            entry.with_context(|| format!("Could not read legacy cache `{}`", source.display()))?;
        let source = entry.path();
        let destination = destination.join(entry.file_name());
        let metadata = entry
            .metadata()
            .with_context(|| format!("Could not inspect legacy cache `{}`", source.display()))?;
        if metadata.is_dir() {
            preserve_modified_times(&source, &destination)?;
        } else {
            let modified = metadata.modified().with_context(|| {
                format!("Could not read cache timestamp `{}`", source.display())
            })?;
            let mut options = File::options();
            options.read(true);
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                use windows_sys::Win32::Storage::FileSystem::FILE_WRITE_ATTRIBUTES;

                options.access_mode(FILE_WRITE_ATTRIBUTES);
            }
            let file = options.open(&destination).with_context(|| {
                format!("Could not open cache file `{}`", destination.display())
            })?;
            file.set_modified(modified).with_context(|| {
                format!(
                    "Could not preserve cache timestamp `{}`",
                    destination.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[cfg_attr(
    dylint_lib = "general",
    allow(
        non_thread_safe_call_in_test,
        reason = "All writes use private temporary directories"
    )
)]
mod tests {
    use super::*;
    use dylint_internal::{library_filename_with_toolchain, library_plain_filename};
    use std::{
        fs::{read, write},
        thread,
        time::UNIX_EPOCH,
    };
    use tempfile::tempdir;

    #[test]
    fn migration_preserves_legacy_cache_and_existing_destination() {
        let root = tempdir().unwrap();
        let source = root.path().join("legacy/toolchain");
        let destination = root.path().join("new/toolchain");
        create_dir_all(source.join("release")).unwrap();
        write(source.join("release/library"), "legacy").unwrap();
        File::options()
            .write(true)
            .open(source.join("release/library"))
            .and_then(|file| file.set_modified(UNIX_EPOCH))
            .unwrap();

        migrate(&source, &destination).unwrap();
        assert_eq!(
            UNIX_EPOCH,
            destination
                .join("release/library")
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap()
        );
        assert_eq!(
            b"legacy",
            read(destination.join("release/library"))
                .unwrap()
                .as_slice()
        );
        assert_eq!(
            b"legacy",
            read(source.join("release/library")).unwrap().as_slice()
        );

        write(source.join("release/library"), "changed legacy").unwrap();
        write(destination.join("release/library"), "new").unwrap();
        migrate(&source, &destination).unwrap();
        assert_eq!(
            b"new",
            read(destination.join("release/library"))
                .unwrap()
                .as_slice()
        );
    }

    #[test]
    fn concurrent_migrations_publish_one_complete_cache() {
        let root = tempdir().unwrap();
        let source = root.path().join("legacy/toolchain");
        let destination = root.path().join("new/toolchain");
        create_dir_all(source.join("release")).unwrap();
        write(source.join("release/.cargo-lock"), "").unwrap();
        write(source.join("release/library"), "library").unwrap();

        thread::scope(|scope| {
            for _ in 0..2 {
                scope.spawn(|| migrate(&source, &destination).unwrap());
            }
        });
        assert_eq!(
            b"library",
            read(destination.join("release/library"))
                .unwrap()
                .as_slice()
        );
    }

    #[test]
    fn missing_legacy_cache_does_not_create_destination() {
        let root = tempdir().unwrap();
        let destination = root.path().join("new/toolchain");
        migrate(&root.path().join("missing"), &destination).unwrap();
        assert!(!destination.exists());
    }

    #[test]
    fn rejected_legacy_cache_is_not_published() {
        let root = tempdir().unwrap();
        let source = root.path().join("legacy/toolchain");
        let destination = root.path().join("new/version/toolchain");
        create_dir_all(&source).unwrap();
        write(source.join("dylint-driver"), "incompatible driver").unwrap();

        migrate_with(&source, &destination, |_| Ok(false)).unwrap();
        assert!(!destination.exists());
        assert!(source.join("dylint-driver").is_file());

        migrate_with(&source, &destination, |_| Ok(true)).unwrap();
        assert!(destination.join("dylint-driver").is_file());
    }

    #[test]
    fn unpartitioned_library_outputs_are_not_published() {
        let root = tempdir().unwrap();
        let source = root.path().join("legacy/toolchain");
        let destination = root.path().join("new/toolchain/library-key");
        let plain = Path::new("release").join(library_plain_filename("library"));
        let tagged =
            Path::new("release").join(library_filename_with_toolchain("library", "toolchain"));
        create_dir_all(source.join("release/deps")).unwrap();
        write(source.join(&plain), "wrong revision").unwrap();
        write(source.join(&tagged), "wrong revision").unwrap();
        write(source.join("release/deps/dependency"), "dependency").unwrap();
        migrate_library(&source, &destination, "library", "toolchain").unwrap();
        assert!(!destination.join(&plain).exists());
        assert!(!destination.join(&tagged).exists());
        assert!(destination.join("release/deps/dependency").is_file());
        assert!(source.join(plain).is_file());
        assert!(source.join(tagged).is_file());
    }
}
