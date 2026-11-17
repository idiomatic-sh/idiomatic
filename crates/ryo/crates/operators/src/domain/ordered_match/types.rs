//! Borrowed ordered match contracts; no engine authority.

/// What an ordered match lowers to: both sides' keys and orders and the output columns.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OrderedParts {
    /// The left keys.
    pub(crate) left_on: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    /// The right keys.
    pub(crate) right_on: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
    /// The left order column.
    pub(crate) left_order: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The right order column.
    pub(crate) right_order: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The output columns.
    pub(crate) output: Vec<ryo_values::domain::plan_parts::types::JoinColumn>,
}
