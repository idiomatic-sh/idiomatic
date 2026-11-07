//! The `hooks` port's failures.

/// Why a hook script could not be found, read, written or removed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum HooksError {
    /// The product's directory is in no git repository.
    #[error("`{path}` is in no git repository")]
    NotARepository {
        /// The directory.
        path: String,
    },
    /// Git could not say where the hook is.
    #[error("git cannot say where hooks are kept: {reason}")]
    Git {
        /// What it said.
        reason: String,
    },
    /// A script could not be read.
    #[error("cannot read `{path}`: {reason}")]
    Unreadable {
        /// The file.
        path: String,
        /// What the system said.
        reason: String,
    },
    /// A script could not be written or removed.
    #[error("cannot write `{path}`: {reason}")]
    Unwritable {
        /// The file.
        path: String,
        /// What the system said.
        reason: String,
    },
}
