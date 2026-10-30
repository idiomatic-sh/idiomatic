//! What the `validate` use case promises its callers.

use super::errors::ValidateError;
use super::types::{ValidateReport, ValidateRequest};

/// What `validate` offers its callers.
pub trait ValidateUseCase: Send + Sync {
    /// Proves the version in use of every definition of the workspace's packages, its latest and any the lock pins: each
    /// explains itself, each rule's query runs, and each scenario passes in a project of its own. A version whose proof
    /// is held is a lookup; only the versions whose digest moved are proved again.
    ///
    /// # Errors
    ///
    /// `Repository`, `Manifest` or `Packages` when the workspace or a package cannot be read, `Analysis` when the engine
    /// fails.
    fn validate(&self, request: ValidateRequest) -> Result<ValidateReport, ValidateError>;
}
