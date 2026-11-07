//! What the `check` use case promises its callers.

use super::errors::CheckError;
use super::types::{CheckReport, CheckRequest};

/// What `check` offers its callers.
pub trait CheckUseCase: Send + Sync {
    /// Checks the workspace nearest the request's start: places every entry, sends one lint call with every rule placed
    /// and, when the plan holds counts or pairs, a second that judges them, and answers the findings, the in-source ignores applied and every finding fingerprinted. Validates nothing.
    ///
    /// # Errors
    ///
    /// `Repository`, `Manifest` or `Packages` when the workspace cannot be read, `Lock` when its lock is stale and the
    /// request does not fix it, `Placement` when an entry's paths do not compile, `Analysis` when the lint call fails.
    fn check(&self, request: CheckRequest) -> Result<CheckReport, CheckError>;
}
