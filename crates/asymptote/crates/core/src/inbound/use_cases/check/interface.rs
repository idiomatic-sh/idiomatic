//! What the `check` use case promises its callers.

use super::errors::CheckError;
use super::types::{CheckReport, CheckRequest};

/// What `check` offers its callers.
pub trait CheckUseCase: Send + Sync {
    /// Runs the product's tools, reusing each run whose inputs did not move, and judges what they report against the
    /// lock, or against what another revision's branch accepted; keeps the report `up` and `down` read.
    ///
    /// # Errors
    ///
    /// `Manifest`, `Lock`, `Fold`, `Repository` or `Store` when the product, its lock, its history or its store cannot
    /// be read or written; `NoHead` for `--since` on a tree with no commit. A tool that fails is no error: its run
    /// says so, and the verdict is undetermined.
    fn check(&self, request: CheckRequest) -> Result<CheckReport, CheckError>;
}
