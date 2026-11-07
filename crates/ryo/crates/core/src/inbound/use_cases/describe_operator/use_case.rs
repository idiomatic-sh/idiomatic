//! The `describe_operator` use case: it finds the operator the name asks for and describes it.

use super::errors::DescribeOperatorError;
use super::interface::DescribeOperatorUseCase;
use super::types::{DescribeOperatorReport, DescribeOperatorRequest};
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use crate::application::metadata::service::kernel_metadata;

/// `describe_operator`; the operators are the kernel's own, asked through the `metadata` application service, so it calls no port.
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
        kernel_metadata()
            .operators()
            .into_iter()
            .map(|operator| kernel_metadata().describe(*operator))
            .find(|description| description.name == request.name)
            .map(|operator| DescribeOperatorReport { operator })
            .ok_or_else(|| DescribeOperatorError::UnknownOperator {
                name: request.name.0.to_string(),
            })
    }
}
