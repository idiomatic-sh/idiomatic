//! Initialization failures.
use crate::outbound::repository::errors::RepositoryError;
/// Why initialization was refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InitError {
    /// A project file would be overwritten.
    #[error("`{file}` already exists: initialization preserves existing assets and configuration")]
    Exists {
        /// The conflicting path.
        file: String,
    },
    /// The project could not be written.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}
