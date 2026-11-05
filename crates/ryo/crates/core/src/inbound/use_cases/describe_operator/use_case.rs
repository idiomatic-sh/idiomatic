//! The `describe_operator` use case: it finds the operator the name asks for and describes it.

use super::errors::DescribeOperatorError;
use super::interface::DescribeOperatorUseCase;
use super::types::{DescribeOperatorReport, DescribeOperatorRequest};
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;

/// `describe_operator`; the operators are the domain's own, so it calls no port.
#[derive(Clone, Copy, Debug, Default)]
pub struct DescribeOperatorUseCaseImpl;

impl DescribeOperatorUseCaseImpl {
    /// `describe_operator`, which calls no port.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl DescribeOperatorUseCase for DescribeOperatorUseCaseImpl {
    fn describe_operator(
        &self,
        request: DescribeOperatorRequest,
    ) -> Result<DescribeOperatorReport, DescribeOperatorError> {
        MetadataUseCaseImpl
            .operators()
            .into_iter()
            .map(|operator| MetadataUseCaseImpl.describe(*operator))
            .find(|description| description.name == request.name)
            .map(|operator| DescribeOperatorReport { operator })
            .ok_or_else(|| DescribeOperatorError::UnknownOperator {
                name: request.name.0.to_string(),
            })
    }
}
