//! The types of the `packages` port: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::packages::types::{PackageRoot, PackageSource, SourcePath};

/// A package found: its directory on the machine, and its source as the lock pins it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located {
    /// Its directory.
    pub directory: PackageRoot,
    /// Its source as the lock records it.
    pub pin: PackageSource,
    /// Where its own dependencies' paths are written from, as the lock's paths read them.
    pub base: SourcePath,
}

/// A file of a package, by its path relative to the package's directory.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageFile(pub Box<str>);

/// A file of a package and the package's directory: what a stamp is asked of.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackagedFile {
    /// The package's directory.
    pub directory: PackageRoot,
    /// The file, relative to it.
    pub file: PackageFile,
}

/// The files of a package the run made and the port holds in memory, each by its path in the package.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeldFiles(
    pub std::collections::BTreeMap<PackageFile, crate::domain::files::types::FileText>,
);

/// The versions of a definition a package holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Versions(pub Vec<crate::domain::packages::types::VersionText>);

/// Files of a package, beneath a directory of it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackageFiles(pub Vec<PackageFile>);

impl std::ops::Deref for Versions {
    type Target = [crate::domain::packages::types::VersionText];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for Versions {
    type Item = crate::domain::packages::types::VersionText;
    type IntoIter = std::vec::IntoIter<crate::domain::packages::types::VersionText>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Versions {
    type Item = &'a crate::domain::packages::types::VersionText;
    type IntoIter = std::slice::Iter<'a, crate::domain::packages::types::VersionText>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl std::ops::Deref for PackageFiles {
    type Target = [PackageFile];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for PackageFiles {
    type Item = PackageFile;
    type IntoIter = std::vec::IntoIter<PackageFile>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a PackageFiles {
    type Item = &'a PackageFile;
    type IntoIter = std::slice::Iter<'a, PackageFile>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
