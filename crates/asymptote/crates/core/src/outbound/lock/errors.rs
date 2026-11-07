//! The `lock` port's failures.

/// Why the lock could not be read or written.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LockError {
    /// A file of it could not be read.
    #[error("cannot read `{path}`: {reason}")]
    Unreadable {
        /// The file.
        path: String,
        /// What the system said.
        reason: String,
    },
    /// An operation is not the shape asymptote writes.
    #[error("operation {operation}: {reason}")]
    Malformed {
        /// The operation.
        operation: String,
        /// What is wrong.
        reason: String,
    },
    /// An operation's text is not what its name addresses: it was edited by hand.
    #[error("operation {expected} holds the text of {actual}: an operation is never edited")]
    Address {
        /// The id its name gives.
        expected: String,
        /// The id its text has.
        actual: String,
    },
    /// An operation could not be written.
    #[error("cannot write `{path}`: {reason}")]
    Unwritable {
        /// The file.
        path: String,
        /// What the system said.
        reason: String,
    },
}
