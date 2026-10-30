//! The `sources` port's failures.

/// Why a file could not be read.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SourcesError {
    /// The path leads outside the product's root.
    #[error("`{path}` lies outside the product")]
    Outside {
        /// The path.
        path: String,
    },
    /// It could not be read as text.
    #[error("cannot read `{path}`: {reason}")]
    Unreadable {
        /// The file.
        path: String,
        /// What the system said.
        reason: String,
    },
}
