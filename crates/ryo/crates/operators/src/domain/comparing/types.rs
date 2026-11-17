//! The types of the `comparing` concept: what a comparison is decided with. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::table_scratch::types::{LazyScratch, SelectScratch};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::PlanComparison;

/// A comparison of a select, with its place among the select's comparisons.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NumberedComparison<'a> {
    /// The comparison's place.
    pub number: ComparisonNumber,
    /// The comparison.
    pub comparison: &'a PlanComparison,
    /// How many comparisons the select has: the length of one name's run of the memo.
    pub comparisons: ComparisonCount,
}

/// What the cells of a row are, and how many names the memo has room for.
#[derive(Clone, Copy, Debug)]
pub struct RowShape<'a> {
    /// The kind of each column of the row.
    pub kinds: &'a [CellKind],
    /// How many names the memo has room for: a name past them is decided without it.
    pub names: NameCount,
}

/// The room a decision is made in: the select's scratch and the memo of what was decided about each name.
#[derive(Debug)]
pub struct DecisionWorkspace<'a> {
    /// The select's scratch: the verdicts kept by text, and the compiled matchers, made only when a comparison
    /// needs them, so a comparison decided by its text alone allocates none.
    pub select: &'a mut LazyScratch<SelectScratch>,
    /// What each comparison decided about each name of the file.
    pub memo: &'a mut crate::domain::table_scratch::types::MemoTable,
}

/// How many names a file has: the number of runs of the memo, one run per name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameCount(pub usize);

/// How many comparisons a select has: the length of one name's run of the memo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ComparisonCount(pub usize);

/// A comparison of a `select`, by its place among the select's comparisons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComparisonNumber(pub usize);
