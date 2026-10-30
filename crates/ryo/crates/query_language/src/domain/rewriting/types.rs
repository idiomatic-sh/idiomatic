//! The types of the `rewriting` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::PlanStep;

/// A step that keeps the rows of `pairs` whose two functions have the same set of texts, said as
/// two antijoins of the texts each way: what it needs to say it as a comparison of two digests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SameSets {
    /// The pairs of functions the sets of which are compared.
    pub pairs: StepId,
    /// The column of `pairs` holding the first function of a pair.
    pub function: ColumnIndex,
    /// The column of `pairs` holding the other.
    pub other: ColumnIndex,
    /// The step holding a function and one of its texts for each row.
    pub texts: StepId,
    /// The column of `texts` holding the function.
    pub owner: ColumnIndex,
    /// The column of `texts` holding the text.
    pub text: ColumnIndex,
}

/// The three columns a join keeps of the pairs joined with their texts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PairColumns {
    /// The first function of a pair.
    pub function: ColumnIndex,
    /// The other.
    pub other: ColumnIndex,
    /// The text.
    pub text: ColumnIndex,
}

/// Whether a projection keeps exactly the first two columns of its input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PairProjection {
    /// It keeps columns 0 and 1, in that order, and no other.
    FirstTwo,
    /// It keeps any other columns.
    Other,
}

/// One direction of the comparison: the antijoin `step` of the rows `(function, other, text)` of a
/// join of the pairs with the texts, against the texts again, kept as the pairs it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ComparisonDirection {
    /// The pairs the direction compares.
    pub pairs: StepId,
    /// The column of the pairs holding the first function.
    pub function: ColumnIndex,
    /// The column of the pairs holding the other.
    pub other: ColumnIndex,
    /// The step holding a function and one of its texts for each row.
    pub texts: StepId,
    /// The column of the texts holding the function.
    pub owner: ColumnIndex,
    /// The column of the texts holding the text.
    pub text: ColumnIndex,
    /// The pair column the texts of the join are of.
    pub joined_on: ColumnIndex,
    /// The column of the joined rows the antijoin asks the texts about, besides the text.
    pub asked: ColumnIndex,
}

/// The steps of a plan, and which of them the query emits.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Planned {
    /// The steps, in order.
    pub steps: Vec<PlanStep>,
    /// The step whose rows answer the query.
    pub emitted: StepId,
}

/// A step's place among the steps of a plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StepIndex(pub usize);
