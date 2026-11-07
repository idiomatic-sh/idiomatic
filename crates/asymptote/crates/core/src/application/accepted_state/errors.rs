//! Failures while preparing accepted state.

use crate::domain::operations::errors::OperationsError;
use crate::outbound::lock::errors::LockError;
use crate::outbound::repository::errors::RepositoryError;

/// Why accepted records cannot be prepared.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AcceptedStateError {
    /// The operation store failed.
    #[error(transparent)]
    Lock(#[from] LockError),
    /// The operation graph is invalid.
    #[error(transparent)]
    Fold(#[from] OperationsError),
    /// The requested history cannot be read.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}
