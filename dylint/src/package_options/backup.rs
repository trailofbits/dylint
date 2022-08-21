use std::{
    fs::{copy, rename},
    io::Result,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

pub struct Backup {
    path: PathBuf,
    tempfile: Option<NamedTempFile>,
}

impl Backup {
    pub fn new<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let tempfile = sibling_tempfile(path.as_ref())?;
        copy(&path, &tempfile)?;
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            tempfile: Some(tempfile),
        })
    }

    pub fn disable(&mut self) -> Result<()> {
        self.tempfile.take().map_or(Ok(()), NamedTempFile::close)
    }
}

impl Drop for Backup {
    fn drop(&mut self) {
        if let Some(tempfile) = self.tempfile.take() {
            rename(&tempfile, &self.path).unwrap_or_default();
        }
    }
}

#[allow(clippy::expect_used)]
fn sibling_tempfile(path: &Path) -> Result<NamedTempFile> {
    let canonical_path = path.canonicalize()?;
    let parent = canonical_path
        .parent()
        .expect("should not fail for a canonical path");
    NamedTempFile::new_in(parent)
}
