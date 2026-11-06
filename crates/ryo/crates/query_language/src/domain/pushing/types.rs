//! The types of the `pushing` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plans::types::Operation;

/// How many steps of a chain of filters are passed over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FilterDepth(pub usize);

/// How many columns come before the ones being counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ColumnPrefixLength(pub usize);

/// What the elements of a lookup are already filtered by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FilteredElements<'a> {
    /// The columns of the elements the keys name, in order.
    pub view: &'a [ColumnIndex],
    /// The comparisons the elements already satisfy.
    pub filter: &'a [PlanComparison],
}

/// What the first join of a lookup keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeptColumns {
    /// The columns of the left the answer reads.
    pub left: Vec<ColumnIndex>,
    /// The columns of the edges the answer and the second join read.
    pub edge: Vec<ColumnIndex>,
}

/// The two joins a lookup is made of.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Pushed {
    /// The join of the left with the edges.
    pub below: Operation,
    /// The join of that with the elements.
    pub above: Operation,
}

/// A filter of a chain of filters: its comparisons, and the columns it keeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FilterStep<'a> {
    /// The comparisons it holds.
    pub comparisons: &'a [PlanComparison],
    /// The columns it keeps; none named means every column.
    pub kept: &'a [ColumnIndex],
}

/// A filter after a capture and what moves out of it: the capture, what it reads, the comparisons that move before it and those that stay.
pub(crate) struct MovedComparisons {
    /// The place of the capture.
    pub capture: crate::domain::rewriting::types::StepIndex,
    /// What the capture reads.
    pub input: Input,
    /// The comparisons that are made before the capture.
    pub before: Vec<PlanComparison>,
    /// The comparisons that stay where they are.
    pub rest: Vec<PlanComparison>,
}
