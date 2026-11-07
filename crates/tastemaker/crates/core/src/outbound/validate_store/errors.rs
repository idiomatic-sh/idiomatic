//! The `validate_store` port's failures.

/// Why the store could not be written.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ValidateStoreError {
    /// The store could not be written.
    #[error("cache not saved at `{file}`: {reason}")]
    Unwritable {
        /// The store's file.
        file: String,
        /// What the system said.
        reason: String,
    },
}
