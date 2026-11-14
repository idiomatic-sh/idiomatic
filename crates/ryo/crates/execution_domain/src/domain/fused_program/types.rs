//! The types of the `fused_program` concept: plain data, public fields and derives, with no constructor and no inherent impl.

pub mod fused_program;
pub mod logical_build;
pub mod program_inputs;

use crate::domain::coverage::types::RuleIndex;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId, ServedAt};
use ryo_values::domain::observations::types::BoundObservationInput;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::{Operation, Plan};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use std::sync::Arc;

/// What a rule's parameters are bound to, by parameter id.
#[derive(Clone, Copy, Debug)]
pub struct RuleBindings<'a>(pub &'a [ParameterBinding]);

/// The schemas of a rule's bound inputs, by binding id.
#[derive(Clone, Copy, Debug)]
pub struct RuleObservations<'a>(pub &'a [BoundObservationInput]);

/// One rule of a request as the logical program reads it: its compiled plan, shared, never copied, and what it is bound to.
#[derive(Clone, Copy, Debug)]
pub struct RuleInput<'a> {
    /// The compiled plan.
    pub plan: &'a Arc<Plan>,
    /// What its parameters are bound to.
    pub bindings: RuleBindings<'a>,
    /// The schemas of its bound inputs.
    pub observations: RuleObservations<'a>,
}

/// Whether a rule's nodes all run at this stage, or the stage that serves the last of them. Branch only: deleted with the last
/// unlowered kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serving {
    /// Every node of the rule runs.
    Served,
    /// The latest stage among the rule's unserved features: a kind with no lowering, a step after the files, or a bound input.
    NotServedUntil(ServedAt),
}

/// A rule's nodes: the node of each of its steps, in step order, and the node it emits, each with the fault of a step whose
/// schema the metadata does not hold.
#[derive(Clone, Debug, PartialEq)]
pub struct RuleNodes {
    /// The node of each step; a step that reads a step after it has none.
    pub steps: Vec<Result<PlanNodeId, OperatingError>>,
    /// The node the rule emits, or the schema fault of it or of a step it reads.
    pub emit: Result<PlanNodeId, OperatingError>,
}

/// How many consumers a node has: node readers, the rules that emit it and the holds that keep it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Consumers(pub u32);

/// One node of the logical program while it is drafted: the operation, the rule it was first written in, its scope and its
/// output's columns.
pub(crate) struct Draft<'p> {
    /// The operation, borrowed from its rule's plan.
    pub(crate) operation: &'p Operation,
    /// The rule that first wrote it.
    pub(crate) rule: RuleIndex,
    /// Where it runs.
    pub(crate) scope: StepScope,
    /// Its output's columns, or the fault of it or of a node it reads.
    pub(crate) schema: Result<Vec<CellKind>, OperatingError>,
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
    /// The stage that serves a bound input among them, if one is.
    pub(crate) stage: Option<ServedAt>,
}
