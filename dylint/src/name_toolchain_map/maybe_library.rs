use anyhow::Result;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MaybeLibrary {
    inner: Inner,
}

impl MaybeLibrary {
    pub fn path(&self) -> PathBuf {
        self.inner.path()
    }

    pub fn build(&self, opts: &crate::opts::Dylint) -> Result<PathBuf> {
        self.inner.build(opts)
    }
}

impl From<PathBuf> for MaybeLibrary {
    fn from(path: PathBuf) -> Self {
        Self {
            inner: Inner::Path(path),
        }
    }
}

#[cfg(__library_packages)]
impl From<crate::library_packages::Package> for MaybeLibrary {
    fn from(package: crate::library_packages::Package) -> Self {
        Self {
            inner: Inner::Package(package),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Inner {
    Path(PathBuf),

    #[cfg(__library_packages)]
    Package(crate::library_packages::Package),
}

impl Inner {
    pub fn path(&self) -> PathBuf {
        match self {
            Self::Path(path) => path.clone(),

            #[cfg(__library_packages)]
            Self::Package(package) => package.path(),
        }
    }

    #[cfg_attr(not(__library_packages), allow(unused_variables))]
    fn build(&self, opts: &crate::opts::Dylint) -> Result<PathBuf> {
        match self {
            Self::Path(path) => Ok(path.clone()),

            #[cfg(__library_packages)]
            Self::Package(package) => crate::library_packages::build_library(opts, package),
        }
    }
}
