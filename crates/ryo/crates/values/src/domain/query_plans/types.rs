//! The types of the `query_plans` concept: a compiled query whole: its parameters, its steps, what it emits and the relations it reads. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::plan_parts::types::ColumnIndex;
use crate::domain::plan_parts::types::Heading;
use crate::domain::plan_parts::types::Input;
use crate::domain::plan_parts::types::StepId;
use crate::domain::plans::types::PlanStep;
use crate::domain::query_bindings::types::PlanParameter;
use crate::domain::query_tokens::types::QueryOrigin;

/// A compiled query: its parameters, its steps in the order to run them, and what it emits.
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Plan {
    /// Ordered schemas of explicit bound inputs; source IDs remain stable through request pruning.
    pub observation_inputs: Vec<crate::domain::observations::types::BoundObservationInput>,
    /// The `param` lines.
    pub parameters: Vec<PlanParameter>,
    /// The steps, each reading only the relations and the steps before it.
    pub steps: Vec<PlanStep>,
    /// Source origins indexed by diagnostic-only comparison IDs.
    pub comparison_origins: Vec<QueryOrigin>,
    /// Explicit named roots; request linking removes undemanded observation-only work.
    pub observations: Vec<crate::domain::observations::types::PlanObservation>,
    /// The `emit` line.
    pub emit: Emit,
    /// What the per-file steps hand to the cross-file steps: the relations and the per-file steps
    /// that a cross-file step reads. Empty when every step is per-file.
    pub frontier: Vec<Input>,
}

/// The relations a rule's steps read, each once, in the order they are first read.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::Relation;
/// use ryo_values::domain::query_plans::types::RuleReads;
/// let reads = RuleReads(Box::new([Relation::Function]));
/// assert!(reads.0.contains(&Relation::Function));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleReads(pub Box<[ryo_relations::domain::relations::types::Relation]>);

/// What the query emits.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Emit {
    /// The step whose rows answer the query.
    pub step: StepId,
    /// The column holding the file a row is about.
    pub subject: ColumnIndex,
    /// The columns that identify a row.
    pub key: Vec<ColumnIndex>,
    /// The headings of the answer's columns: text that leaves the engine.
    pub headings: Vec<Heading>,
}
