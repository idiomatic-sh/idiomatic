//! The `lint_store` port's failures.

/// Why an operation of `store` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LintStoreError {
    /// The store could not be read.
    #[error("cannot read the store in `{directory}`: {reason}")]
    Unreadable {
        /// The directory the store is in.
        directory: String,
        /// What went wrong.
        reason: String,
    },
    /// The store could not be written.
    #[error("cannot write the store in `{directory}`: {reason}")]
    Unwritable {
        /// The directory the store is in.
        directory: String,
        /// What went wrong.
        reason: String,
    },
    /// A keep needed words of a store the run did not read, and would have copied them from whatever is there.
    #[error("the run keeps words of a store it did not read")]
    NothingRead,
}
