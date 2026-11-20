//! The names of the `query_operators` concept: the one list of every operator.

use super::types::QueryOperator;

/// Every operator, in the order the documentation lists them.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::query_operators::constants::OPERATORS;
/// use ryo_values::domain::query_operators::types::QueryOperator;
/// assert!(OPERATORS.contains(&QueryOperator::AsOf));
/// ```
pub const OPERATORS: [QueryOperator; 31] = [
    QueryOperator::SpanBetween,
    QueryOperator::SpanBounds,
    QueryOperator::SourceMatches,
    QueryOperator::RenderSyntax,
    QueryOperator::FitsRendered,
    QueryOperator::Replace,
    QueryOperator::OrderedMatch,
    QueryOperator::AsOf,
    QueryOperator::OrderedDfs,
    QueryOperator::Worklist,
    QueryOperator::Select,
    QueryOperator::Project,
    QueryOperator::Extend,
    QueryOperator::Join,
    QueryOperator::Union,
    QueryOperator::Intersection,
    QueryOperator::Difference,
    QueryOperator::Antijoin,
    QueryOperator::GroupCount,
    QueryOperator::GroupDigest,
    QueryOperator::GroupMin,
    QueryOperator::GroupMax,
    QueryOperator::GroupOrdinal,
    QueryOperator::Capture,
    QueryOperator::Words,
    QueryOperator::Rank,
    QueryOperator::Axis,
    QueryOperator::Rows,
    QueryOperator::Closure,
    QueryOperator::Follow,
    QueryOperator::Cycles,
];
