//! The `describe_operator` use case: it finds the operator the name asks for and describes it.

use super::errors::DescribeOperatorError;
use super::interface::DescribeOperatorUseCase;
use super::types::{DescribeOperatorReport, DescribeOperatorRequest};
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use std::sync::Arc;

/// `describe_operator`, over the metadata use case that knows the operators.
pub struct DescribeOperatorUseCaseImpl {
    /// Knows the operators and describes each.
    metadata: Arc<dyn MetadataUseCase + Send + Sync>,
}

impl std::fmt::Debug for DescribeOperatorUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl DescribeOperatorUseCaseImpl {
    /// `describe_operator` over the metadata use case the edge injects.
    #[must_use]
    pub fn new(metadata: Arc<dyn MetadataUseCase + Send + Sync>) -> Self {
        Self { metadata }
    }
}

impl DescribeOperatorUseCase for DescribeOperatorUseCaseImpl {
    fn describe_operator(
        &self,
        request: DescribeOperatorRequest,
    ) -> Result<DescribeOperatorReport, DescribeOperatorError> {
        self.metadata
            .operators()
            .into_iter()
            .map(|operator| self.metadata.describe(*operator))
            .find(|description| description.name == request.name)
            .map(|operator| DescribeOperatorReport { operator })
            .ok_or_else(|| DescribeOperatorError::UnknownOperator {
                name: request.name.0.to_string(),
            })
    }
}
