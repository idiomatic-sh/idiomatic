//! The `open` use case's failures.

/// Why `open` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OpenError {
    /// The relation catalog could not be read.
    #[error("the relation catalog failed: {reason}")]
    Language {
        /// What the language port said.
        reason: String,
    },
    /// The memory budget cannot be had.
    #[error("memory: {reason}")]
    Memory {
        /// Why.
        reason: String,
    },
}
