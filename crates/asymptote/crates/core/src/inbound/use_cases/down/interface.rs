//! What the `down` use case promises its callers.

use super::errors::DownError;
use super::types::{DownReport, DownRequest};

/// What `down` offers its callers.
pub trait DownUseCase: Send + Sync {
    /// Drains the lock of every record the last check found fixed, lowering a record some of whose violations still
    /// occur, in one operation; runs no tool.
    ///
    /// # Errors
    ///
    /// `NoReport`, `HeadMoved`, `Moved` or `ToolsFailed` when the last check no longer holds or could not judge;
    /// `Manifest`, `Lock`, `Fold`, `Repository` or `Store` when the product cannot be read or written.
    fn down(&self, request: DownRequest) -> Result<DownReport, DownError>;
}
