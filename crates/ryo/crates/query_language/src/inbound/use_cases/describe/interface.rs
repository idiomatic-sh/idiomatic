//! What the `describe` use case promises its callers.

use super::errors::DescribeError;
use super::types::DescribeReport;

/// What `describe` offers its callers.
pub trait DescribeUseCase: Send + Sync {
    /// The grammar of RyoQL: every production, from the whole program down to a word, each with text it reads.
    ///
    /// # Errors
    ///
    /// Never fails.
    fn describe(&self) -> Result<DescribeReport, DescribeError>;
}
