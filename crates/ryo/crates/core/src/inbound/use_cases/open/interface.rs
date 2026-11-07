//! What the `open` use case promises its callers.

use super::errors::OpenError;
use super::types::{OpenReport, OpenRequest};

/// What `open` offers its callers.
pub trait OpenUseCase: Send + Sync {
    /// Opens a session over a repository: its root, its workers and its memory budget.
    ///
    /// # Errors
    ///
    /// `Language` when the catalog cannot be read, `Memory` when the machine cannot give the budget.
    fn open(&self, request: OpenRequest) -> Result<OpenReport, OpenError>;
}
