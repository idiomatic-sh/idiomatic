//! What a projection lowers to.

use ryo_values::domain::plan_parts::types::{ColumnIndex, RepeatedRows};

/// The columns a projection keeps, in order, and whether a repeated row is kept once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectParts {
    /// The columns, in order.
    pub(crate) columns: Vec<ColumnIndex>,
    /// Whether a repeated row is kept once.
    pub(crate) distinct: RepeatedRows,
}
