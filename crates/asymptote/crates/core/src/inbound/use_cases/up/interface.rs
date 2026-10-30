//! What the `up` use case promises its callers.

use super::errors::UpError;
use super::types::{UpReport, UpRequest};

/// What `up` offers its callers.
pub trait UpUseCase: Send + Sync {
    /// Records the new violations of the last check that the selectors choose, with the reason; runs no tool.
    ///
    /// # Errors
    ///
    /// `NoReport`, `HeadMoved`, `Moved` or `ToolsFailed` when the last check no longer holds or could not judge;
    /// `NothingAt` when an `--at` names no new violation; `Unkeyed` when one chosen has no fingerprint; `Manifest`,
    /// `Lock`, `Fold`, `Repository` or `Matching` when the product cannot be read or written.
    fn up(&self, request: UpRequest) -> Result<UpReport, UpError>;
}
