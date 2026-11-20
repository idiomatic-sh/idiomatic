//! The types of the `distinctness` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;

use crate::domain::lowering::types::Uniqueness;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::StepId;

/// What has been found of the distinctness of the steps of one plan, by step and set of columns: a step shared by many readers is
/// followed once. Good only while the plan is unchanged: a rewrite that moves steps starts a new one.
#[derive(Clone, Debug, Default)]
pub(crate) struct DistinctMemo {
    /// What was found of each set of columns of a step followed so far.
    pub found: BTreeMap<StepKeys, Uniqueness>,
    /// How many sets of columns of steps were followed into what their step reads: each once, answered from `found` after.
    pub followed: KeysFollowed,
}

/// How many sets of columns of steps a memo found by following the plan, not from the memo: the distinctness cases hold it to one a
/// set, so a plan whose steps meet again is followed in steps, not in paths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KeysFollowed(pub usize);

/// A set of columns of a step of the plan, in ascending order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StepKeys {
    /// The step.
    pub step: StepId,
    /// Its columns, each once, in ascending order.
    pub columns: Vec<ColumnIndex>,
}
