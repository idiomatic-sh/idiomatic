//! The types of the `narrowing` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::PlanStep;

/// A per-file input of a join after the files and the steps that narrow it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Narrowed {
    /// The per-file input.
    pub local: Input,
    /// Its columns the join reads, in order: the columns the narrowed rows keep.
    pub kept: Vec<ColumnIndex>,
    /// The steps that narrow it, to stand just before the join, the last of which is the narrowed rows.
    pub made: Vec<PlanStep>,
}
