//! Select's predicate outcome and its answer, and what a select lowers to.

use ryo_values::domain::plan_parts::types::{ColumnIndex, PlanComparison};
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// What a select keeps: its comparisons, its kept columns and what its parameters are bound to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SelectParts {
    /// The comparisons, in their order.
    pub(crate) comparisons: Vec<PlanComparison>,
    /// The columns kept; none means every column.
    pub(crate) kept: Vec<ColumnIndex>,
    /// What the comparisons' parameters are bound to.
    pub(crate) bindings: Vec<ParameterBinding>,
}
