//! What the `scaffold` use case promises its callers.

use super::errors::ScaffoldError;
use super::types::{ScaffoldReport, ScaffoldRequest};

/// What `scaffold` offers its callers.
pub trait ScaffoldUseCase: Send + Sync {
    /// Writes the files an entry places that do not exist yet, each rendered from its template; never overwrites. A
    /// spec entry writes the files of one instance at `at`, a template entry its own file.
    ///
    /// # Errors
    ///
    /// `NoEntry` when no entry has the name, `Capture` when a path still captures, `Render` when a template does not
    /// render, and the workspace's failures when it cannot be read.
    fn scaffold(&self, request: ScaffoldRequest) -> Result<ScaffoldReport, ScaffoldError>;
}
