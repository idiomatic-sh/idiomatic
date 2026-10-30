//! `operator describe`: composes, calls the service once.

use ryo_core::inbound::use_cases::describe_operator::interface::DescribeOperatorUseCase as _;

use super::errors::DescribeError;
use super::schemas;
use super::types::{DescribeReport, DescribeRequest};

/// `operator describe`: describes the one operator asked for.
///
/// # Errors
///
/// `UnknownOperator` when no operator has the name.
pub fn describe(request: DescribeRequest) -> Result<DescribeReport, DescribeError> {
    super::composition::build()
        .describe_operator(schemas::request(request))
        .map(schemas::report)
        .map_err(schemas::error)
}
