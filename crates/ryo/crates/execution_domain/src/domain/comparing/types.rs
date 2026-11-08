//! The types of the `comparing` concept: what a comparison is decided with. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operating::types::{ComparisonNumber, Memo, NameCount};
use crate::domain::table_scratch::types::SelectScratch;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::PlanComparison;

/// A comparison of a select, with its place among the select's comparisons.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NumberedComparison<'a> {
    /// The comparison's place.
    pub number: ComparisonNumber,
    /// The comparison.
    pub comparison: &'a PlanComparison,
}

/// What the cells of a row are, and how many names the file has.
#[derive(Clone, Copy, Debug)]
pub struct RowShape<'a> {
    /// The kind of each column of the row.
    pub kinds: &'a [CellKind],
    /// How many names the file has: the length of a comparison's run of the memo.
    pub names: NameCount,
}

/// The room a decision is made in: the select's scratch and the memo of what was decided about each name.
#[derive(Debug)]
pub struct DecisionWorkspace<'a> {
    /// The select's scratch: the verdicts kept by text, and the compiled matchers.
    pub select: &'a mut SelectScratch,
    /// What each comparison decided about each name of the file.
    pub memo: &'a mut Vec<Memo>,
}
