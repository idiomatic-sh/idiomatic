//! The lowering of an ordered match; the pairing is the `ordered_match` kernel's (Spike ryo-fmx3).
use ryo_values::domain::plan_parts::types::JoinColumn;

/// The one lowering of an ordered match: both sides' keys and orders and its output columns, for the `ordered_match` kernel.
pub(crate) fn lower(
    left_on: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    right_on: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    left_order: ryo_values::domain::plan_parts::types::ColumnIndex,
    right_order: ryo_values::domain::plan_parts::types::ColumnIndex,
    output: &[JoinColumn],
) -> super::types::OrderedParts {
    super::types::OrderedParts {
        left_on: left_on.to_vec(),
        right_on: right_on.to_vec(),
        left_order,
        right_order,
        output: output.to_vec(),
        band: None,
    }
}
