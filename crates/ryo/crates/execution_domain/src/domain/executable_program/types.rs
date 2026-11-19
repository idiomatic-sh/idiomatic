//! The types of the `executable_program` concept: plain data, public fields and derives, with no constructor and no inherent impl.

pub mod executable_program;
pub mod logical_build;
pub mod program_inputs;

use crate::domain::coverage::types::RuleIndex;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use ryo_operators::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::observations::types::BoundObservationInput;
use ryo_values::domain::plan_parts::types::{ColumnIndex, LiteralText, QueryLiteral, StepScope};
use ryo_values::domain::plans::types::{Operation, Plan};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use std::sync::Arc;

/// What a rule's parameters are bound to, by parameter id.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::types::RuleBindings;
/// assert!(RuleBindings(&[]).0.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct RuleBindings<'a>(pub &'a [ParameterBinding]);

/// The schemas of a rule's bound inputs, by binding id.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::types::RuleObservations;
/// assert!(RuleObservations(&[]).0.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct RuleObservations<'a>(pub &'a [BoundObservationInput]);

/// One rule of a request as the logical program reads it: its compiled plan, shared, never copied, and what it is bound to.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::types::{RuleBindings, RuleInput, RuleObservations};
/// use ryo_values::domain::plans::types::Plan;
/// fn rule(plan: &std::sync::Arc<Plan>) -> RuleInput<'_> {
///     RuleInput { plan, bindings: RuleBindings(&[]), observations: RuleObservations(&[]) }
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct RuleInput<'a> {
    /// The compiled plan.
    pub plan: &'a Arc<Plan>,
    /// What its parameters are bound to.
    pub bindings: RuleBindings<'a>,
    /// The schemas of its bound inputs.
    pub observations: RuleObservations<'a>,
}

/// A rule's nodes: the node of each of its steps, in step order, and the node it emits, each with the fault of a step whose
/// schema the metadata does not hold.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::types::RuleNodes;
/// use ryo_operators::domain::physical_plan::types::PlanNodeId;
/// let nodes = RuleNodes { steps: vec![Ok(PlanNodeId(0))], emit: Ok(PlanNodeId(0)), refused: None };
/// assert_eq!(nodes.emit, Ok(PlanNodeId(0)));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct RuleNodes {
    /// The node of each step; a step that reads a step after it has none.
    pub steps: Vec<Result<PlanNodeId, OperatingError>>,
    /// The node the rule emits, or the schema fault of it or of a step it reads.
    pub emit: Result<PlanNodeId, OperatingError>,
    /// The schema fault of the first step no root reads, in step order: it has no root to answer it, so it stops the rule's
    /// run, in each file and after the files, before any row.
    pub refused: Option<OperatingError>,
}

/// How many consumers a node has: node readers, the rules that emit it and the holds that keep it.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::types::Consumers;
/// assert_eq!(Consumers::default(), Consumers(0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Consumers(pub u32);

/// A root kind of a program: the text the plans write for it and the nodes that start from the tree nodes of that kind.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::types::RootKind;
/// use ryo_values::domain::plan_parts::types::LiteralText;
/// let kind = LiteralText("function_definition".into());
/// assert!(RootKind { kind: &kind, nodes: &[] }.nodes.is_empty());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RootKind<'a> {
    /// The kind's text, as the plans write it.
    pub kind: &'a LiteralText,
    /// The nodes that start from it, in node order.
    pub nodes: &'a [PlanNodeId],
}

/// One node of the logical program while it is drafted: the operation, the rule it was first written in, its scope and its
/// output's columns.
pub(crate) struct Draft<'p> {
    /// The operation, borrowed from its rule's plan.
    pub(crate) operation: &'p Operation,
    /// The rule that first wrote it.
    pub(crate) rule: RuleIndex,
    /// Where it runs.
    pub(crate) scope: StepScope,
    /// Its output's columns as checked, or the fault of it or of a node it reads.
    pub(crate) schema: Result<Vec<CellKind>, OperatingError>,
    /// The columns its rows answer with as a root: its schema, or, for a refusal deferred to its rows, the layout its empty
    /// rows keep (the operator contract's `output_layout`).
    pub(crate) layout: Result<Vec<CellKind>, OperatingError>,
    /// When its schema's refusal, if any, is answered.
    pub(crate) timing: SchemaTiming,
}

/// When a drafted node's schema refusal is answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SchemaTiming {
    /// Its schema is as drafted: its columns, or a refusal answered before any row.
    Stated,
    /// Its own refusal is its kernel's, at the first row that reaches it: as a root it answers its layout, and a node reading
    /// it reads no column of it.
    DeferredToRows,
    /// Its refusal is answered once the nodes it reads have run, after any refusal of theirs: a step reading a node refuses
    /// after its inputs, as the interpreter did, and one reading a refusal deferred to its rows reads no column of it.
    AfterInputs,
}

/// What a step's inputs read: the node inputs, their columns, the first schema fault among the nodes they read, and the stage
/// that serves a bound input among them.
pub(crate) struct DraftInputs<'s> {
    /// The inputs, in the order the step reads them.
    pub(crate) list: Vec<PlanInput>,
    /// Their columns.
    pub(crate) shapes: Vec<InputShape<'s>>,
    /// The first schema fault of a node they read.
    pub(crate) fault: Option<OperatingError>,
    /// `AfterInputs` when a node they read defers its refusal or answers it after its own inputs; `Stated` otherwise.
    pub(crate) timing: SchemaTiming,
}

/// Where a step being drafted runs, the rule it is a step of and that rule's plan: what its inputs are read as.
#[derive(Clone, Copy)]
pub(crate) struct StepAt<'p> {
    /// Where it runs.
    pub(crate) scope: StepScope,
    /// The rule.
    pub(crate) rule: RuleOrdinal,
    /// The rule's plan.
    pub(crate) plan: &'p Plan,
}

/// Where a rule's emitted step runs and where each of its named outputs is made.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::types::RuleScopes;
/// use ryo_values::domain::plan_parts::types::StepScope;
/// let scopes = RuleScopes { emit: StepScope::CrossFile, observations: Vec::new() };
/// assert_eq!(scopes.emit, StepScope::CrossFile);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleScopes {
    /// Where the emitted step runs.
    pub emit: StepScope,
    /// Where each named output is made, in output order.
    pub observations: Vec<StepScope>,
}

/// Whether a node's rows answer the whole phase after the files, or one occurrence: a node that reads a bound observation
/// answers the slots its occurrence bound, an antijoin or a difference after the files keeps the rows its occurrence's coverage
/// vouches for, and a node that reads either answers its occurrence too.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::types::ObservationReach;
/// assert_ne!(ObservationReach::Phase, ObservationReach::Occurrence);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationReach {
    /// Its rows are the same for every occurrence of the phase.
    Phase,
    /// Its rows answer one occurrence: its bound observations or its coverage.
    Occurrence,
}

/// Whether one of a plan's roots (its emitted step, a frontier entry or a named output) reads a step, through the steps that
/// read it.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::types::StepUse;
/// assert_ne!(StepUse::Read, StepUse::Unread);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepUse {
    /// A root reads the step.
    Read,
    /// No root reads the step: its fault has no root to answer it.
    Unread,
}

/// What drafting a step answers of its columns: its schema as checked, the columns its rows answer with as a root, and when
/// its refusal, if any, is answered.
pub(crate) struct DraftedColumns {
    /// Its output's columns as checked, or its refusal.
    pub(crate) schema: Result<Vec<CellKind>, OperatingError>,
    /// The columns its rows answer with as a root.
    pub(crate) layout: Result<Vec<CellKind>, OperatingError>,
    /// When its refusal is answered.
    pub(crate) timing: SchemaTiming,
}

/// One total literal equality in the original source's columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SourceViewBranch<'p> {
    /// The input column of the equality.
    pub(crate) column: ColumnIndex,
    /// The checked literal.
    pub(crate) value: &'p QueryLiteral,
}

/// A stable first-distinct projection over one source, with ordered branches.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ProjectedSourceRows<'p> {
    /// The only relation acquired.
    pub(crate) relation: Relation,
    /// The output columns in their authored order.
    pub(crate) columns: Vec<ColumnIndex>,
    /// The first scan's predicate.
    pub(crate) first: SourceViewBranch<'p>,
    /// The optional second scan's predicate.
    pub(crate) second: Option<SourceViewBranch<'p>>,
}

/// The operation and actual inputs of a per-file node whose preflight succeeded.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CheckedSourceNode<'s, 'p> {
    /// The original compiled operation.
    pub(crate) operation: &'p Operation,
    /// Its resolved input slots.
    pub(crate) inputs: &'s [PlanInput],
}

/// The literal source of a filter and any mapping into its physical columns.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SourceColumnMap<'p> {
    /// The catalog relation requested by this view.
    pub(crate) relation: Relation,
    /// Each row's projected columns, or the identity when the row reads its source directly.
    pub(crate) projection: Option<&'p [ColumnIndex]>,
}

/// A typed condition under which a complete source predicate is empty in a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceAbsence {
    /// Source whose rows the file delivered completely.
    pub relation: Relation,
    /// Physical column carrying the predicate's text.
    pub column: ColumnIndex,
    /// Text absent from that column throughout the file.
    pub text: LiteralText,
}
