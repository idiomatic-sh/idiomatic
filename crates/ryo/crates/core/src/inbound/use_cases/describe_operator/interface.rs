//! What the `describe_operator` use case promises its callers.

use super::errors::DescribeOperatorError;
use super::types::{DescribeOperatorReport, DescribeOperatorRequest};

/// What `describe_operator` offers its callers.
pub trait DescribeOperatorUseCase: Send + Sync {
    /// Describes the one operator asked for.
    ///
    /// # Errors
    ///
    /// `UnknownOperator` when no operator has the name.
    fn describe_operator(
        &self,
        request: DescribeOperatorRequest,
    ) -> Result<DescribeOperatorReport, DescribeOperatorError>;
}
