//! The `facts` port's failures.

/// Why the `facts` port failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FactsError {
    /// Where the rows or the words are kept refused or failed.
    #[error("the facts store failed: {reason}")]
    Store {
        /// What it said.
        reason: String,
    },
    /// A file the words are kept in failed.
    #[error("a file of the facts failed: {reason}")]
    File {
        /// What the system said.
        reason: String,
    },
}
