//! The types of the `table_dedup` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use crate::domain::table_model::types::Addition;

/// Where a row is among the rows of a table, and whether it was added by the call that looked for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RowFound {
    /// The row.
    pub at: RowIndex,
    /// Whether it was not there before.
    pub added: Addition,
}

use ryo_values::domain::rows::types::RowIndex;
