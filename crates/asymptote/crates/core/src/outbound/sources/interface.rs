//! What the `sources` port asks of the outside, in the core's own vocabulary.

use super::errors::SourcesError;
use crate::domain::reports::types::{FileDigests, Stats};
use crate::domain::violations::types::{Located, RepoPath, SourceText};

/// What every `sources` adapter implements.
pub trait SourcesPort: std::fmt::Debug + Send + Sync {
    /// The text of `path`, relative to the product.
    ///
    /// # Errors
    ///
    /// `Outside` when it leads outside the product, `Unreadable` when it cannot be read; bytes that are no UTF-8 are
    /// read lossily.
    fn read(&self, path: &RepoPath) -> Result<SourceText, SourcesError>;

    /// What each of `paths`, relative to the product, is now, read from its metadata alone: its size and modification
    /// time, or that it is absent; and the moment they were looked at, taken before any was.
    fn stats(&self, paths: &[RepoPath]) -> Stats;

    /// The digest of each of `paths`' bytes, relative to the product, reading each: the absent digest for one that is gone.
    fn digests(&self, paths: &[RepoPath]) -> FileDigests;

    /// Where `path`, relative to the product, leads once its links are followed: outside the product, or not.
    fn locate(&self, path: &RepoPath) -> Located;
}
