//! What the `describe_relation` use case promises its callers.

use super::errors::DescribeRelationError;
use super::types::{DescribeRelationReport, DescribeRelationRequest};

/// What `describe_relation` offers its callers.
pub trait DescribeRelationUseCase: Send + Sync {
    /// Describes the one relation asked for.
    ///
    /// # Errors
    ///
    /// `UnknownRelation` when ryo owns no relation of that name, `Language` when the catalog cannot be read.
    fn describe_relation(
        &self,
        request: DescribeRelationRequest,
    ) -> Result<DescribeRelationReport, DescribeRelationError>;
}
