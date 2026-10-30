//! The types of the `selections` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::{ColumnIndex, RepeatedRows};

/// What a projection keeps.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProjectedColumns<'a> {
    /// The columns kept, in order.
    pub columns: &'a [ColumnIndex],
    /// Whether the rows that repeat are dropped.
    pub repeats: RepeatedRows,
}
