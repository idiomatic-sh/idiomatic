//! The `repository` port's failures.

/// Why what a repository holds could not be read as the port's types.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RepositoryError {
    /// A name or a text is not text.
    #[error("not text")]
    NotText,
}
