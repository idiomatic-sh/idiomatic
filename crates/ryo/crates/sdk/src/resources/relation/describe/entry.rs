//! `relation describe`: composes, calls the service once.

use ryo_core::inbound::use_cases::describe_relation::interface::DescribeRelationUseCase as _;

use super::errors::DescribeError;
use super::schemas;
use super::types::{DescribeReport, DescribeRequest};
use crate::domain::language::types::Languages;

/// `relation describe`: describes one relation ryo owns.
///
/// # Errors
///
/// `UnknownRelation` when ryo owns no relation of that name.
pub fn describe(
    languages: Languages,
    request: DescribeRequest,
) -> Result<DescribeReport, DescribeError> {
    super::composition::build(languages.0)
        .describe_relation(schemas::request(request))
        .map(schemas::report)
        .map_err(schemas::error)
}
