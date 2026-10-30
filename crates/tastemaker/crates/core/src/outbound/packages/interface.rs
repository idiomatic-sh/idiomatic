//! What the `packages` port asks of the outside, in the core's own vocabulary: where a package is, and its files.

use super::errors::PackagesError;
use super::types::{Located, PackageFile};
use crate::domain::files::types::FileText;
use crate::domain::packages::types::{Dependency, PackageRoot, SourcePath};

/// What every `packages` adapter implements.
pub trait PackagesPort: Send + Sync {
    /// Where `dependency` is, written in the file at `from`, whose paths the lock reads from `base`.
    ///
    /// # Errors
    ///
    /// `Unsupported` for a source this build does not resolve, `Missing` when its directory is not there.
    fn locate(
        &self,
        dependency: &Dependency,
        from: &PackageRoot,
        base: &SourcePath,
    ) -> Result<Located, PackagesError>;

    /// A file of the package at `directory`.
    ///
    /// # Errors
    ///
    /// `Read` when it cannot be read.
    fn read(&self, directory: &PackageRoot, file: &PackageFile) -> Result<FileText, PackagesError>;

    /// Whether the package at `directory` offers `file`: it is there and can be read, and nothing of it is kept.
    ///
    /// # Errors
    ///
    /// `Read` when it is not there or cannot be read.
    fn offers(&self, directory: &PackageRoot, file: &PackageFile) -> Result<(), PackagesError>;

    /// The versions the package offers of one definition: the names of the directories at `within`, none when there is no
    /// such directory.
    ///
    /// # Errors
    ///
    /// `Read` when the directory is there but cannot be listed, or a name in it is not UTF-8.
    fn versions(
        &self,
        directory: &PackageRoot,
        within: &PackageFile,
    ) -> Result<super::types::Versions, PackagesError>;

    /// Every file beneath `within` of the package at `directory`, relative to `within`, sorted; none when there is no such
    /// directory.
    ///
    /// # Errors
    ///
    /// `Read` when a directory beneath it cannot be listed, or a name in it is not UTF-8.
    fn files(
        &self,
        directory: &PackageRoot,
        within: &PackageFile,
    ) -> Result<super::types::PackageFiles, PackagesError>;

    /// What each file's size and modification time come to, in order; an empty stamp for one that is gone.
    fn stamps(
        &self,
        files: &[crate::outbound::packages::types::PackagedFile],
    ) -> crate::domain::files::types::Stamps;

    /// Holds a package the run made in memory, for as long as the adapter lives, and answers the directory it is read
    /// at: its files, by their paths in it, are read, listed and located as any package's are, and written nowhere.
    fn hold(&self, files: super::types::HeldFiles) -> PackageRoot;
}
