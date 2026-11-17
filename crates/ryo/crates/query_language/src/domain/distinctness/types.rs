//! The types of the `distinctness` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;

use crate::domain::lowering::types::Uniqueness;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::StepId;

/// What has been found of the distinctness of the steps of one plan, by step and set of columns: a step shared by many readers is
/// followed once. Good only while the plan is unchanged: a rewrite that moves steps starts a new one.
#[derive(Clone, Debug, Default)]
pub(crate) struct DistinctMemo(pub BTreeMap<StepKeys, Uniqueness>);

/// A set of columns of a step of the plan, in ascending order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StepKeys {
    /// The step.
    pub step: StepId,
    /// Its columns, each once, in ascending order.
    pub columns: Vec<ColumnIndex>,
}

/// Whether a join's keys are the file alone: every key column of its left side is the file the row lies in, so the join pairs
/// every row of a file with every row of it on the other side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileKeyed {
    /// Every key is the file, and there is one.
    Alone,
    /// A key is something other than the file, or there is none.
    Not,
}
