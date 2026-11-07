//! The `check_store` port's failures.

/// Why the store could not be kept.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CheckStoreError {
    /// Its directory could not be written.
    #[error("cannot keep the store in `{directory}`: {reason}")]
    Unwritable {
        /// The directory.
        directory: String,
        /// What the system said.
        reason: String,
    },
}
