//! The types of the `selection_pushing` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::rewriting::types::StepIndex;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::PlanStep;

/// What takes the place of one filter of a union or a join: the filter's place, the steps made before it, in order, and what it
/// becomes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PushedFilter {
    /// The place of the filter.
    pub reader: StepIndex,
    /// The filters of the inputs and, when the filter stays, the union or the join of them: each reads only the steps before the
    /// filter and the ones before it here.
    pub before: Vec<PlanStep>,
    /// The filter's own step.
    pub filter: PlanStep,
}

/// The comparisons of a filter of a join, by where they are decided: those of the filter's infallible prefix (`fixed_prefix`) that
/// test the columns of one input, on that input, and on the other input too when they test only keys of the join, which hold the
/// same values there; an equality of a left and a right column as a key of the join; and the rest after the join. A comparison
/// that may fail, and every one after it, is never moved, so each runs on the rows it ran on and raises the fault it raised.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct SplitComparisons {
    /// Those of the prefix that test only columns of the left input, as its columns.
    pub left: Vec<PlanComparison>,
    /// Those of the prefix that test only columns of the right input, as its columns.
    pub right: Vec<PlanComparison>,
    /// The rest, in their order, as columns of the join's answer: those of the prefix that test no single input, then every
    /// comparison from the first that may fail on.
    pub rest: Vec<PlanComparison>,
    /// The columns of the left input that the equalities of the prefix between a left and a right column name, as further
    /// keys of the join, in their order.
    pub left_on: Vec<ColumnIndex>,
    /// The columns of the right input, as the join's view places them, that the same equalities name, in the same order.
    pub right_on: Vec<ColumnIndex>,
}

/// The inputs, the keys, the answer and the right view of a join, as a filter of it is split by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Keyed<'a> {
    /// The left input.
    pub left: Input,
    /// The right input.
    pub right: Input,
    /// The left key columns.
    pub left_on: &'a [ColumnIndex],
    /// The right key columns, as the view places them.
    pub right_on: &'a [ColumnIndex],
    /// The columns of the answer.
    pub output: &'a [JoinColumn],
    /// The columns of the right input the keys and the answer name, in order; none named means all of them.
    pub view: &'a [ColumnIndex],
}

/// What a filter of a step becomes below it: the steps made before it, in order, the step that takes the step's place for the
/// filter, and the comparisons of the filter that were placed below, as the step's own columns.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Below {
    /// The filters of the step's inputs: each reads only the steps before the filter and the ones before it here.
    pub before: Vec<PlanStep>,
    /// The step made again over the filtered inputs.
    pub combined: PlanStep,
    /// The comparisons placed below.
    pub moved: Vec<PlanComparison>,
}

/// What is left of one filter of a step once the comparisons every filter of it shares are placed below the step: the filter's
/// place, and the comparisons and the columns it still has.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Residual {
    /// The place of the filter.
    pub reader: StepIndex,
    /// The comparisons it still makes, in their order.
    pub comparisons: Vec<PlanComparison>,
    /// The columns it keeps.
    pub kept: Vec<ColumnIndex>,
    /// Where it runs.
    pub scope: StepScope,
    /// The statement it was written as.
    pub written: ryo_values::domain::written_sites::types::StepWritten,
}

/// The comparisons every filter of a step shares, placed below the step once, and what each filter keeps of its own.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SharedPush {
    /// The step made again below the filters, and what is made before it.
    pub below: Below,
    /// Each filter of the step, from the first.
    pub residuals: Vec<Residual>,
}

/// A key of a join that an equality of a column of each input makes: the left input's column, and the right input's, as the
/// join's view places it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CrossingKey {
    /// The column of the left input.
    pub left: ColumnIndex,
    /// The column of the right input, as the view places it.
    pub right: ColumnIndex,
}

/// The hash of a step of a plan, its operation and where it runs, by which a step that repeats an earlier one is found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct StepHash(pub u64);

/// Where a deferred splice put its steps (prototype ryo-fmx3.1): `count` steps made before the step at `at`, put at the
/// end of the plan from `base`; the step at `at` was replaced where it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SplicePlaces {
    /// The place of the step they were made before.
    pub at: usize,
    /// How many were made.
    pub count: usize,
    /// Where the first stands.
    pub base: usize,
}
