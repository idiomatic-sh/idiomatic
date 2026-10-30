//! The `check` use case's failures.

use crate::domain::operations::errors::OperationsError;
use crate::outbound::check_store::errors::CheckStoreError;
use crate::outbound::lock::errors::LockError;
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::repository::errors::RepositoryError;

/// Why `check` could not judge the product.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CheckError {
    /// The product's `asymptote.toml` could not be found or read.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// The lock could not be read.
    #[error(transparent)]
    Lock(#[from] LockError),
    /// The lock's operations do not fold.
    #[error(transparent)]
    Fold(#[from] OperationsError),
    /// Git could not answer.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// The store could not be written.
    #[error(transparent)]
    Store(#[from] CheckStoreError),
    /// `--since` needs a commit to compare from, and the working tree stands on none.
    #[error("`--since` compares from a commit, and the working tree stands on none")]
    NoHead,
}

impl From<crate::application::accepted_state::errors::AcceptedStateError> for CheckError {
    fn from(error: crate::application::accepted_state::errors::AcceptedStateError) -> Self {
        use crate::application::accepted_state::errors::AcceptedStateError;
        match error {
            AcceptedStateError::Lock(error) => Self::Lock(error),
            AcceptedStateError::Fold(error) => Self::Fold(error),
            AcceptedStateError::Repository(error) => Self::Repository(error),
        }
    }
}
