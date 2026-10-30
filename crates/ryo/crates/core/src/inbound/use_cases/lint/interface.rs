//! What the `lint` use case promises its callers.

use super::errors::LintError;
use super::types::{LintReport, LintRequest};

/// What `lint` offers its callers.
pub trait LintUseCase: Send + Sync {
    /// Checks the rules over the session's repository and writes the findings, as rows of typed
    /// cells (the rule, the subject, the key values and the headed columns, then the place), to the
    /// rows port it was given, by file and, in a file, by rule and key; the answer is the files
    /// nothing read.
    ///
    /// # Errors
    ///
    /// `Rule` for a rule that is wrong; `Corpus`, `Facts`, `Language`, `Memory`, `Scheduler` when a port fails; `Output` and `OutputClosed` when the findings cannot be written.
    fn lint(&self, request: LintRequest) -> Result<LintReport, LintError>;
}
