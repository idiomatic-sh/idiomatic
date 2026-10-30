//! What the `explain` use case promises its callers.

use super::errors::ExplainError;
use super::types::{ExplainReport, ExplainRequest};

/// What `explain` offers its callers.
pub trait ExplainUseCase: Send + Sync {
    /// What applies to a path and what must be there, what a definition is, or a topic; or the whole workspace. Writes
    /// nothing, but for `--all --output`.
    ///
    /// # Errors
    ///
    /// `Unknown` when nothing has the name asked, and the workspace's failures when it cannot be read or placed.
    fn explain(&self, request: ExplainRequest) -> Result<ExplainReport, ExplainError>;
}
