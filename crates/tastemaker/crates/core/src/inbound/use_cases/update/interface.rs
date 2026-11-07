//! What the `update` use case promises its callers.

use super::errors::UpdateError;
use super::types::{UpdateReport, UpdateRequest};

/// What `update` offers its callers.
pub trait UpdateUseCase: Send + Sync {
    /// Moves the root file's entries to the latest version their package offers, or to the version asked, keeping the
    /// file's layout; with `-n` it lists what would move. The lock is rewritten by the next `check --fix`.
    ///
    /// # Errors
    ///
    /// `NoEntry` when the entry asked for is not there, and the workspace's failures when it cannot be read or written.
    fn update(&self, request: UpdateRequest) -> Result<UpdateReport, UpdateError>;
}
