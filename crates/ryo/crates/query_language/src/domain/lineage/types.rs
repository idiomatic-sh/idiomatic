//! The types of the `lineage` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;

use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::lineage::types::Lineage;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::StepId;

/// A column of a per-file step whose values a column after the files holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ValueOrigin {
    /// The per-file step.
    pub input: Input,
    /// Its column.
    pub column: ColumnIndex,
}

/// What has been found of the steps of one plan, by step: a step shared by many readers is followed once, so following the columns
/// of a plan whose steps meet again grows with its steps, not with its paths. Good only while the plan is unchanged: a rewrite that
/// moves steps starts a new one.
#[derive(Clone, Debug, Default)]
pub(crate) struct LineageMemo {
    /// The kind of each column of a step followed so far.
    pub kinds: BTreeMap<StepColumn, Option<CellKind>>,
    /// The per-file origins of each column of a step followed so far.
    pub origins: BTreeMap<StepColumn, Option<Vec<ValueOrigin>>>,
    /// The lineage of each step found so far.
    pub lineages: BTreeMap<StepId, Lineage>,
}

/// A column of a step of the plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StepColumn {
    /// The step.
    pub step: StepId,
    /// Its column.
    pub column: ColumnIndex,
}
