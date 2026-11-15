//! PredicateOutcome outcome precedes kept-column reads, including malformed unused columns.
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The one lowering of a select: its comparisons, kept columns and bindings, for the `filter` kernel.
pub(crate) fn lower(
    comparisons: &[ryo_values::domain::plan_parts::types::PlanComparison],
    kept: &[ColumnIndex],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
) -> super::types::SelectParts {
    super::types::SelectParts {
        comparisons: comparisons.to_vec(),
        kept: kept.to_vec(),
        bindings: bindings.to_vec(),
    }
}
