//! The failures of `relation list`.

/// Why `list` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ListError {
    /// The relation catalog could not be read.
    #[error("the relation catalog failed: {reason}")]
    Language {
        /// What the language port said.
        reason: String,
    },
}
