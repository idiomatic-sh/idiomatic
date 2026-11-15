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
    /// The select's scratch: the verdicts kept by text, and the compiled matchers, made only when a comparison
    /// needs them, so a comparison decided by its text alone allocates none.
    pub select: &'a mut LazyScratch<SelectScratch>,
    /// What each comparison decided about each name of the file.
    pub memo: &'a mut Vec<crate::domain::table_scratch::types::Memo>,
}

/// The families of comparison a `select` tests with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// Equal and not equal.
    Equality,
    /// Less, greater, and their or-equal.
    Ordering,
    /// Numbers written as text.
    FloatText,
    /// Exact canonical numeric text and whole numbers.
    Numeric,
    /// In and not in.
    Membership,
    /// Glob matches.
    Glob,
    /// Starts with and ends with.
    Affix,
    /// Regular expression searches.
    Regex,
}

/// How many names a file has: the length of a comparison's run of the memo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameCount(pub usize);

/// A comparison of a `select`, by its place among the select's comparisons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComparisonNumber(pub usize);

/// The leading comparisons of a select that no row can make fail with an error: each tests a column the row has against a
/// text the query wrote, by equality or by affix, so its cell's text alone decides it. The rest of the select's comparisons
/// follow it in their order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedPrefix<'a>(pub &'a [PlanComparison]);
