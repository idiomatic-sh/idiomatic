//! What the `list_relations` use case promises its callers.

use super::errors::ListRelationsError;
use super::types::ListRelationsReport;

/// What `list_relations` offers its callers.
pub trait ListRelationsUseCase: Send + Sync {
    /// Lists every relation ryo owns.
    ///
    /// # Errors
    ///
    /// `Language` when the catalog cannot be read.
    fn list_relations(&self) -> Result<ListRelationsReport, ListRelationsError>;
}
