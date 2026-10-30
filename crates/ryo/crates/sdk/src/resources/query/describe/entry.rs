//! `query describe`: composes, calls the service once.

use ryo_query_language::inbound::use_cases::describe::interface::DescribeUseCase as _;

use super::errors::DescribeError;
use super::schemas;
use super::types::DescribeReport;

/// `query describe`: the grammar of RyoQL, every production from the whole program down to a word, each with text it reads.
/// It needs no language.
///
/// # Errors
///
/// Never fails.
pub fn describe() -> Result<DescribeReport, DescribeError> {
    super::composition::build()
        .describe()
        .map(schemas::report)
        .map_err(schemas::error)
}
