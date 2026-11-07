//! What the `add` use case promises its callers.

use super::errors::AddError;
use super::types::{AddReport, AddRequest};

/// What `add` offers its callers.
pub trait AddUseCase: Send + Sync {
    /// Writes one entry into the workspace's root `tastemaker.toml`, at the definition's latest version, its layout kept.
    ///
    /// # Errors
    ///
    /// `NoDefinition` when no package holds it, and the workspace's failures when it cannot be read or written.
    fn add(&self, request: AddRequest) -> Result<AddReport, AddError>;
}
