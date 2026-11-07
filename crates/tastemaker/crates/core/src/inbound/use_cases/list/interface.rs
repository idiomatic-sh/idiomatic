//! What the `list` use case promises its callers.

use super::errors::ListError;
use super::types::{ListReport, ListRequest};

/// What `list` offers its callers.
pub trait ListUseCase: Send + Sync {
    /// The definitions of the workspace's packages, each at its latest version, with what it is.
    ///
    /// # Errors
    ///
    /// `Repository`, `Manifest` or `Packages` when the workspace or a package cannot be read.
    fn list(&self, request: ListRequest) -> Result<ListReport, ListError>;
}
