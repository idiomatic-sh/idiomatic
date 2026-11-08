//! The types of the `selection_pushing` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plans::types::PlanStep;

/// What takes the place of one filter of a union or a join: the steps made before it, in order, and what it becomes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PushedFilter {
    /// The filters of the inputs and, when the filter stays, the union or the join of them: each reads only the steps before the
    /// filter and the ones before it here.
    pub before: Vec<PlanStep>,
    /// The filter's own step.
    pub filter: PlanStep,
}

/// The comparisons of a filter of a join, by the input whose columns they test.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct SplitComparisons {
    /// Those that test only columns of the left input, as its columns.
    pub left: Vec<PlanComparison>,
    /// Those that test only columns of the right input, as its columns.
    pub right: Vec<PlanComparison>,
    /// The rest, as columns of the join's answer.
    pub rest: Vec<PlanComparison>,
}

/// Whether filters made before a step pay for a copy of it for each filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilterGain {
    /// The step pairs every row of a file with every row of it, or unites such rows: the filters move before it.
    Pays,
    /// Anything else: the filters stay where they are.
    Stays,
}

/// Whether a comparison tests a column of the rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tested {
    /// It names a column.
    AColumn,
    /// It names constants only.
    ConstantsOnly,
}
