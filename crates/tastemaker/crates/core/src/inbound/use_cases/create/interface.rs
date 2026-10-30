//! What the `create` use case promises its callers.

use super::errors::CreateError;
use super::types::{CreateReport, CreateRequest};

/// What `create` offers its callers.
pub trait CreateUseCase: Send + Sync {
    /// Writes the skeleton of a new definition at its first version in the package.
    ///
    /// # Errors
    ///
    /// `NoPackage` when there is no package to write into, `Exists` when the definition is there.
    fn create(&self, request: CreateRequest) -> Result<CreateReport, CreateError>;
}
