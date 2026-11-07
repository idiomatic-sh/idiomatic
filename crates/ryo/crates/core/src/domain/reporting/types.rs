//! The types of the `reporting` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. What a run counts when its report is written.

use ryo_values::domain::row_texts::types::RowTotal;

/// How many rows a query has and how many of them are written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowCounts {
    /// Every row, whatever the limit.
    pub total: RowTotal,
    /// The rows that are written: those up to the limit.
    pub shown: RowTotal,
}
