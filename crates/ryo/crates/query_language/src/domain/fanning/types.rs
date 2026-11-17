//! The types of the `fanning` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, KeyFan};

/// What a join pairs its sides on, as the plan writes it: each side and its key columns, the right's read through the view, and
/// the fan the query wrote.
#[derive(Clone, Copy, Debug)]
pub(crate) struct JoinKeys<'a> {
    /// The left side.
    pub left: Input,
    /// The left key columns.
    pub left_on: &'a [ColumnIndex],
    /// The right side.
    pub right: Input,
    /// The right key columns, as the join names them.
    pub right_on: &'a [ColumnIndex],
    /// Where each right column the join names is in the right rows.
    pub view: &'a [ColumnIndex],
    /// The fan as written: `AllPairs` for an `all-pairs join`.
    pub written: KeyFan,
}
