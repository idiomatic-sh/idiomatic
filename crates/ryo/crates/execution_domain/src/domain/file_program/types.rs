//! Immutable request graph and bounded, invocation-local shared results.

use crate::domain::self_join_recogniser::types::{Decline, Shape};
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

pub(crate) mod canonical_operation;
pub(crate) mod fingerprint;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ProgramNodeId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NodeInput {
    Relation(Relation),
    Node(ProgramNodeId),
    /// A named input reads already-published producer roots, never demands them under this rule.
    Observation(ObservationGraphInputId),
}

#[derive(Debug)]
pub(crate) struct FileProgramNode {
    pub needs: Result<
        Vec<crate::domain::trust::types::RelationRead>,
        crate::domain::operating::errors::OperatingError,
    >,
    pub phase: ryo_values::domain::plan_parts::types::StepScope,
    /// One semantic input and dependency contract for all output delivery.
    pub contract: Arc<crate::domain::operator::contract::types::OperatorContract>,
    /// Executable registered operation with normalized local input numbers.
    pub operation: ryo_values::domain::plans::types::Operation,
    /// Ordered typed dependencies retained for the central executor.
    pub inputs: Vec<NodeInput>,
    /// Invalid synthetic plans keep ordinary error order instead of DAG admission.
    pub schema_valid: SchemaValidity,
}

#[derive(Debug)]
pub(crate) struct RequestGraph {
    /// Every original step, including terminal and project steps. Invalid
    /// references are deferred faults, never a choice of another scheduler.
    pub program_nodes: Vec<Vec<Result<ProgramNodeId, crate::domain::operating::errors::OperatingError>>>,
    /// Reviewed alternatives indexed by their replaced Select producer.
    pub ordered_existence: Vec<Option<ResolvedAlternative>>,
    /// Named roots extend this request graph; ordinary requests retain no observation allocation.
    pub observations: Option<GraphObservations>,
    /// Producers of declared derived inputs, computed once for the whole request.
    pub derived_sources: DerivedSources,
    pub nodes: Vec<FileProgramNode>,
}

/// Ordinary requests need no named-root traversal allocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RuleSchedule {
    /// Preserve the existing independent original-index traversal.
    Independent,
    /// Original indices traversed once in dependency order and ready batches.
    Dependencies {
        order: Vec<crate::domain::coverage::types::RuleIndex>,
        batches: Vec<Vec<crate::domain::coverage::types::RuleIndex>>,
    },
}
/// A producer identity resolved once during request compilation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedObservationProducer {
    /// An actual named root in a compiled producer plan.
    Output {
        rule: crate::domain::coverage::types::RuleIndex,
        output: ryo_values::domain::records::types::ObservationIndex,
    },
    /// Explicit unavailable selected query, never a fake empty root.
    NoQuery {
        rule: ryo_values::domain::runs::types::RuleId,
    },
}
/// A resolved consumer input in its stable local slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedObservationBinding {
    /// Local compiled input identity.
    pub input: ryo_values::domain::plan_parts::types::ObservationBindingId,
    /// The actual inferred schema and phase.
    pub schema: ryo_values::domain::observations::types::BoundObservationInput,
    /// Actual typed producers; no runtime name lookup remains.
    pub producers: Vec<ResolvedObservationProducer>,
}
/// Compile-time descriptors and their resolved graph references travel together.
pub(crate) struct PreparedObservationInputs {
    /// Compiler declarations in original local input order.
    pub schemas: Vec<ryo_values::domain::observations::types::BoundObservationInput>,
    /// Corresponding resolved graph roots.
    pub bindings: Vec<ResolvedObservationBinding>,
}

/// The resolved request dependency contract borrowed by the existing file/cross graph owners.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ObservationRequest<'a> {
    /// No bound named roots: keep independent scheduling and no observation state.
    Independent,
    /// Actual resolved bindings and original-index topology, never a second executor.
    Bound {
        layout: ryo_values::domain::records::types::ObservationReadinessLayout,
        bindings: &'a [Vec<ResolvedObservationBinding>],
    },
}

/// Stable local bound input qualified by its original consumer index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ObservationSlot {
    pub rule: crate::domain::coverage::types::RuleIndex,
    pub input: ryo_values::domain::plan_parts::types::ObservationBindingId,
}
/// Actual prepared root location; unsupported admission never becomes complete-empty.
#[derive(Clone, Debug)]
pub(crate) enum GraphObservationRoot {
    PerFile(ProgramNodeId),
    CrossFile,
    Unavailable,
}
/// One named root and its actual final-consumer accounting.
#[derive(Clone, Debug)]
pub(crate) struct GraphObservationOutput {
    pub root: GraphObservationRoot,
    pub consumers: Vec<crate::domain::coverage::types::RuleIndex>,
}
/// The existing request graph owns both typed binding edges and root lifetime declarations.
#[derive(Debug)]
pub(crate) struct GraphObservations {
    pub layout: ryo_values::domain::records::types::ObservationReadinessLayout,
    /// Request-qualified bound slots; graph edges carry a compact ID rather than widening every ordinary edge.
    pub slots: Vec<ObservationSlot>,
    /// Frontier buckets actually needed by authored observations, prepared once.
    pub observation_frontiers: Vec<Vec<crate::domain::rule_intent::types::FrontierPurpose>>,
    /// Logical output status slots in stable rule/output order; aliases may share a physical bucket.
    pub frontiers: Vec<ObservedFrontierRoot>,
    /// Prepared outgoing retirement edges, indexed by original consumer rule.
    pub retire: Vec<Vec<ObservationRetirement>>,
    pub bindings: Vec<Vec<GraphObservationBinding>>,
    pub outputs: Vec<Vec<GraphObservationOutput>>,
}

/// One resolved input and its request-prepared distinct-set combination contract.
#[derive(Debug)]
pub(crate) struct GraphObservationBinding {
    pub input: ResolvedObservationBinding,
    pub union: Result<
        Arc<crate::domain::operator::contract::types::OperatorContract>,
        crate::domain::operating::errors::OperatingError,
    >,
}

/// One unique consumer-to-producer-output lifetime edge, resolved once per request.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ObservationRetirement {
    pub producer: crate::domain::coverage::types::RuleIndex,
    pub output: ryo_values::domain::records::types::ObservationIndex,
}

/// A persisted named root uses an existing frontier bucket, not another row store.
#[derive(Clone, Copy, Debug)]
pub struct ObservedFrontierRoot {
    pub producer: crate::domain::coverage::types::RuleIndex,
    pub output: ryo_values::domain::records::types::ObservationIndex,
    pub frontier: ryo_values::domain::plan_parts::types::InputNumber,
}

/// One request-wide binding slot; its payload is the same width as an ordinary ProgramNodeId edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ObservationGraphInputId(pub usize);

/// Request-owned binding to canonical producers, never template Step(0) equality.
#[derive(Clone, Debug)]
pub(crate) struct ResolvedShape {
    pub shape: Shape,
    pub source: ProgramNodeId,
    pub join: ProgramNodeId,
    pub select: ProgramNodeId,
    /// Exact logical output layout consumed by the unchanged Project.
    pub output: Vec<ryo_values::domain::plan_parts::types::JoinColumn>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedAlternative {
    Eligible(ResolvedShape),
    Declined(Decline),
}

/// Where the binding slots of one rule start among the binding slots of the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingOffset(pub usize);

/// How many bound inputs a plan declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingCount(pub usize);

/// The steps of one plan that linking keeps, by the step's place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WantedSteps(pub Vec<bool>);

/// The named outputs of one plan that a consumer reads, by the output's place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ExportedOutputs(pub Vec<bool>);

/// The named outputs of one plan that a cross-file step reads, by the output's place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EscapingOutputs(pub Vec<bool>);

/// The bound inputs of one rule that a kept step reads, by the input's place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct UsedInputs(pub Vec<bool>);

/// A step of one rule that linking has still to follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PendingStep {
    /// The rule the step belongs to.
    pub rule: crate::domain::coverage::types::RuleIndex,
    /// The step to follow.
    pub step: ryo_values::domain::plan_parts::types::StepId,
}

/// The nodes of a request graph that a derived view reads, by the node's place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DerivedSources(pub Vec<bool>);

/// Whether the contract of a node and of everything it reads has a valid schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SchemaValidity {
    /// Every schema is valid.
    Valid,
    /// A schema is invalid; ordinary error order applies instead of DAG admission.
    Invalid,
}
