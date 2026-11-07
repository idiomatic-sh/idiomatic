//! The `language` port's failures.

/// Why the `language` port failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LanguageError {
    /// The relation catalog could not be read.
    #[error("the relation catalog is unavailable: {reason}")]
    Catalog {
        /// What went wrong.
        reason: String,
    },
    /// A manifest does not parse.
    #[error("a manifest failed: {reason}")]
    Manifest {
        /// What went wrong.
        reason: String,
    },
}
