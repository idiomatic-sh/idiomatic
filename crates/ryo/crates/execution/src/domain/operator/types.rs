//! Immutable plan layouts and shared execution accounting vocabulary.
use ryo_relations::domain::relations::types::CellKind;

/// How many rows a step was fed: the rows of each of its inputs, by the input's place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowsFed(pub [u64; 3]);

/// How many rows each step of a rule was fed, by the step's place.
pub struct LogicalCounts {
    pub fed: Box<[RowsFed]>,
}

/// How a plan is laid out for the worker that runs it: which steps keep their rows in a table, and what columns each step answers.
#[derive(Debug, Default)]
pub(crate) struct PlanLayout {
    /// Which steps keep their rows in the pool's table of the step: those the plan emits or hands over, and those more than one
    /// step reads that are not cheap to read again.
    pub kept: KeptSteps,
    /// The kinds of the columns each step answers.
    pub kinds: Vec<Vec<CellKind>>,
    /// The steps that run over the positions of the tree, not as operators.
    pub lowering: crate::domain::lowering::types::Lowering,
}

/// A step of a plan, by its place among the plan's steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StepPlace(pub usize);

/// A plan, by where it is: the same plan has the same identity in every file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PlanIdentity(pub usize);

/// Which steps keep their rows in the pool's table of the step, by the step's place.
#[derive(Debug, Default)]
pub(crate) struct KeptSteps(pub Vec<bool>);
