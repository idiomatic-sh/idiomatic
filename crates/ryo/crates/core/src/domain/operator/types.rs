//! The types of the `operator` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use ryo_relations::domain::relations::types::{CellKind, Value};

use crate::domain::lowering::types::Counting;

use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::Room;
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::operator::collections::operators::interface::OperatorsItem;
use crate::domain::table_model::types::ParameterBinding;
use ryo_values::domain::plan_parts::types::Inputs;
use ryo_values::domain::plans::types::{Operation, OperationKind};

/// Mutable tables and operator rooms of one rule invocation, dropped after rendering.
#[derive(Debug, Default)]
pub(crate) struct Pool {
    /// The rows of each step that keeps them.
    pub steps: Vec<Table>,
    /// The rows of each input handed to the cross-file steps.
    pub frontier: Vec<Table>,
    /// The relations the file being run is read in part: a step that answers from the absence of their rows makes none.
    pub partial: Vec<crate::domain::coverage::types::Partial>,
}

/// Request-owned immutable schemas/programs, literal matchers and counters; no row payload.
#[derive(Debug, Default)]
pub(crate) struct PreparedPool {
    /// Opt-in execution strategy selected once for this request's workers.
    pub kernel_strategy: crate::domain::column_kernel::types::KernelStrategy,
    /// Pure shape declarations only; runtime strategy requires admitted numeric provenance.
    pub ordered_existence: std::sync::Arc<Vec<crate::domain::ordered_existence::types::Program>>,
    /// Finite authored literal matcher slots shared by this worker's rule/file invocations.
    pub matchers: std::sync::Arc<crate::domain::matching::types::prepared::value::PreparedMatchers>,
    /// One exact active vocabulary and immutable literal-kind metadata; replaced on vocabulary change.
    pub kind_vocabulary:
        Option<std::sync::Arc<crate::domain::kind_sets::types::PreparedVocabulary>>,
    /// Pure stored selector programs, compiled once under this worker's request bindings.
    pub selection_plans: std::sync::Arc<Vec<crate::domain::stored_selection::types::PreparedPlan>>,
    /// Immutable request-known derived view capabilities for those exact selector Arcs.
    pub selection_views:
        Option<std::sync::Arc<crate::domain::stored_selection::types::PreparedSelectionViews>>,
    /// Whether the steps that read only the tree run lowered, or as operators, as they did before they were lowered.
    pub unlowered: TreeSteps,
    /// Whether the rows each step is fed are counted.
    pub counting: Counting,
    /// For each plan this pool has run, in the order of `layouts`, how many rows each of its steps was fed over all the files.
    pub fed: Vec<Vec<RowsFed>>,
}

/// One rule invocation's mutable workspace. Rust drops it after rendering, on success or error.
#[derive(Debug, Default)]
pub(crate) struct RuleWorkspace {
    /// Tables, rooms, and lowered temporary positions for this rule only.
    pub pool: Pool,
    /// Row-dependent scratch and dynamic matcher caches have this exact invocation lifetime.
    pub scratch: crate::domain::table_scratch::types::OperatorScratch,
    /// Comparisons remembered only within this invocation.
    pub memo: Vec<crate::domain::operating::types::Memo>,
}

/// How many rows a step was fed: the rows of each of its inputs, by the input's place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowsFed(pub [u64; 3]);

/// How a plan is laid out for the worker that runs it: which steps keep their rows in a table, and what columns each step answers.
#[derive(Debug, Default)]
pub(crate) struct PlanLayout {
    /// The plan, by where it is: the same plan has the same layout in every file.
    #[cfg(test)]
    pub plan: PlanIdentity,
    /// Which steps keep their rows in the pool's table of the step: those the plan emits or hands over, and those more than one
    /// step reads that are not cheap to read again.
    pub kept: KeptSteps,
    /// Whether a later operation consumes this result, excluding emit/frontier exports.
    #[cfg(test)]
    pub consumed: Vec<bool>,
    /// The kinds of the columns each step answers.
    pub kinds: Vec<Vec<CellKind>>,
    /// The steps that run over the positions of the tree, not as operators.
    pub lowering: crate::domain::lowering::types::Lowering,
    /// For each step, the relations whose absence of rows it trusts; empty when it trusts none.
    #[cfg(test)]
    pub needs: Vec<Vec<crate::domain::trust::types::RelationRead>>,
}

/// A step of a plan, by its place among the plan's steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StepPlace(pub usize);

/// How many steps read a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg(test)]
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

use ryo_values::domain::rows::types::Table;
