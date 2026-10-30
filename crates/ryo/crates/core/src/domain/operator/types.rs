//! The types of the `operator` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use ryo_language::domain::relations::types::{CellKind, Value};

use crate::domain::lowering::types::Counting;
use crate::domain::table_model::types::Table;

use crate::domain::operating::types::Room;

/// The tables one rule computes in, for one worker: one for each step and one for each input
/// handed to the cross-file steps. They are emptied and filled again for each file, so computing
/// allocates nothing once they have grown to the largest file.
#[derive(Debug, Default)]
pub(crate) struct Pool {
    /// The rows of each step that keeps them.
    pub steps: Vec<Table>,
    /// The rows of each input handed to the cross-file steps.
    pub frontier: Vec<Table>,
    /// What each step keeps for itself.
    pub rooms: Vec<Option<Box<Room>>>,
    /// How each plan this pool has run is laid out.
    pub layouts: Vec<PlanLayout>,
    /// Rows being read one at a time from a relation, one buffer for each scan at once.
    pub spare: Vec<Vec<Value>>,
    /// The rows of the lowered steps of the plan being run, by step, for the file being run.
    pub made: crate::domain::positions::types::LoweredRoom,
    /// Whether the steps that read only the tree run lowered, or as operators, as they did before they were lowered.
    pub unlowered: TreeSteps,
    /// Whether the rows each step is fed are counted.
    pub counting: Counting,
    /// The relations the file being run is read in part: a step that answers from the absence of their rows makes none.
    pub partial: Vec<crate::domain::coverage::types::Partial>,
    /// For each plan this pool has run, in the order of `layouts`, how many rows each of its steps was fed over all the files.
    pub fed: Vec<Vec<RowsFed>>,
}

/// How many rows a step was fed: the rows of each of its inputs, by the input's place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowsFed(pub [u64; 3]);

/// How a plan is laid out for the worker that runs it: which steps keep their rows in a table, and what columns each step answers.
#[derive(Debug, Default)]
pub(crate) struct PlanLayout {
    /// The plan, by where it is: the same plan has the same layout in every file.
    pub plan: PlanIdentity,
    /// Which steps keep their rows in the pool's table of the step: those the plan emits or hands over, and those more than one
    /// step reads that are not cheap to read again.
    pub kept: KeptSteps,
    /// The kinds of the columns each step answers.
    pub kinds: Vec<Vec<CellKind>>,
    /// The steps that run over the positions of the tree, not as operators.
    pub lowering: crate::domain::lowering::types::Lowering,
    /// For each step, the relations whose absence of rows it trusts; empty when it trusts none.
    pub needs: Vec<Vec<crate::domain::trust::types::RelationRead>>,
}

/// How many slots a capture pattern has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SlotCount(pub usize);

/// A step of a plan, by its place among the plan's steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StepPlace(pub usize);

/// How many steps read a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ReadCount(pub usize);

/// A plan, by where it is: the same plan has the same identity in every file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PlanIdentity(pub usize);

/// How the steps that read only the tree run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TreeSteps {
    /// Over the positions of the tree.
    #[default]
    Lowered,
    /// As operators, as they did before they were lowered.
    AsOperators,
}

/// Which steps keep their rows in the pool's table of the step, by the step's place.
#[derive(Debug, Default)]
pub(crate) struct KeptSteps(pub Vec<bool>);
