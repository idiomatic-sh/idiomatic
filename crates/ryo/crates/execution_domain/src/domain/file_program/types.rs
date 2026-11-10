//! Immutable request graph and bounded, invocation-local shared results.

use crate::domain::self_join_recogniser::types::{Decline, Shape};
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

pub(crate) mod canonical_operation;
pub(crate) mod fingerprint;

/// A node of the per-file program, by its place among the request graph's nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProgramNodeId(pub usize);

/// What a node of the per-file program reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeInput {
    /// A relation the language provides.
    Relation(Relation),
    /// The output of another node.
    Node(ProgramNodeId),
    /// A named input reads already-published producer roots, never demands them under this rule.
    Observation(ObservationGraphInputId),
}

/// One node of the per-file program: the reads it trusts the absence of, its phase and its contracts.
#[derive(Debug)]
pub struct FileProgramNode {
    /// The reads whose absence of rows the node trusts, or why they could not be known.
    pub needs: Result<
        Vec<crate::domain::trust::types::RelationRead>,
        crate::domain::operating::errors::OperatingError,
    >,
    /// Whether the node runs for each file or for the project.
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

/// The request's operations compiled into one graph: its observations, its derived sources and its nodes.
#[derive(Debug)]
pub struct RequestGraph {
    /// Every original step, including terminal and project steps. Invalid
    /// references are deferred faults, never a choice of another scheduler.
    pub program_nodes:
        Vec<Vec<Result<ProgramNodeId, crate::domain::operating::errors::OperatingError>>>,
    /// Reviewed alternatives indexed by their replaced Select producer.
    pub ordered_existence: Vec<Option<ResolvedAlternative>>,
    /// Named roots extend this request graph; ordinary requests retain no observation allocation.
    pub observations: Option<GraphObservations>,
    /// Producers of declared derived inputs, computed once for the whole request.
    pub derived_sources: DerivedSources,
    /// The nodes of the graph, by node.
    pub nodes: Vec<FileProgramNode>,
}

/// Stable local bound input qualified by its original consumer index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObservationSlot {
    /// The consuming rule.
    pub rule: crate::domain::coverage::types::RuleIndex,
    /// The binding of the rule's input.
    pub input: ryo_values::domain::plan_parts::types::ObservationBindingId,
}
/// Actual prepared root location; unsupported admission never becomes complete-empty.
#[derive(Clone, Debug, Copy)]
pub enum GraphObservationRoot {
    /// The root is produced for each file, by this node.
    PerFile(ProgramNodeId),
    /// The root is produced across files.
    CrossFile,
    /// The root could not be prepared.
    Unavailable,
}
/// One named root and its actual final-consumer accounting.
#[derive(Clone, Debug)]
pub struct GraphObservationOutput {
    /// Where the root is produced.
    pub root: GraphObservationRoot,
    /// The rules that consume the root last.
    pub consumers: Vec<crate::domain::coverage::types::RuleIndex>,
}
/// The existing request graph owns both typed binding edges and root lifetime declarations.
#[derive(Debug)]
pub struct GraphObservations {
    /// Where each rule's observations are made ready.
    pub layout: ryo_values::domain::records::types::ObservationReadinessLayout,
    /// Request-qualified bound slots; graph edges carry a compact ID rather than widening every ordinary edge.
    pub slots: Vec<ObservationSlot>,
    /// Frontier buckets actually needed by authored observations, prepared once.
    pub observation_frontiers: Vec<Vec<crate::domain::rule_intent::types::FrontierPurpose>>,
    /// Logical output status slots in stable rule/output order; aliases may share a physical bucket.
    pub frontiers: Vec<crate::domain::observation_schedule::types::ObservedFrontierRoot>,
    /// Prepared outgoing retirement edges, indexed by original consumer rule.
    pub retire: Vec<Vec<ObservationRetirement>>,
    /// The resolved input bindings of each rule, by rule.
    pub bindings: Vec<Vec<GraphObservationBinding>>,
    /// The named outputs of each rule, by rule.
    pub outputs: Vec<Vec<GraphObservationOutput>>,
}

/// One resolved input and its request-prepared distinct-set combination contract.
#[derive(Debug)]
pub struct GraphObservationBinding {
    /// The resolved input.
    pub input: crate::domain::observation_schedule::types::ResolvedObservationBinding,
    /// The contract that combines the input's producers into one distinct set, or why it could not be prepared.
    pub union: Result<
        Arc<crate::domain::operator::contract::types::OperatorContract>,
        crate::domain::operating::errors::OperatingError,
    >,
}

/// One unique consumer-to-producer-output lifetime edge, resolved once per request.
#[derive(Clone, Copy, Debug)]
pub struct ObservationRetirement {
    /// The producing rule.
    pub producer: crate::domain::coverage::types::RuleIndex,
    /// The producer's named output.
    pub output: ryo_values::domain::records::types::ObservationIndex,
}

/// One request-wide binding slot; its payload is the same width as an ordinary ProgramNodeId edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObservationGraphInputId(pub usize);

/// Request-owned binding to canonical producers, never template Step(0) equality.
#[derive(Clone, Debug)]
pub struct ResolvedShape {
    /// The recognised shape.
    pub shape: Shape,
    /// The node the shape reads.
    pub source: ProgramNodeId,
    /// The join node the shape replaces.
    pub join: ProgramNodeId,
    /// The select node the shape replaces.
    pub select: ProgramNodeId,
    /// Exact logical output layout consumed by the unchanged Project.
    pub output: Vec<ryo_values::domain::plan_parts::types::JoinColumn>,
}

/// What the recogniser resolved for a self join.
#[derive(Clone, Debug)]
pub enum ResolvedAlternative {
    /// An ordered existence the engine can run.
    Eligible(ResolvedShape),
    /// Why the self join stays a join.
    Declined(Decline),
}

/// Where the binding slots of one rule start among the binding slots of the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingOffset(pub usize);

/// The nodes of a request graph that a derived view reads, by the node's place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DerivedSources(pub Vec<bool>);

/// Whether the contract of a node and of everything it reads has a valid schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaValidity {
    /// Every schema is valid.
    Valid,
    /// A schema is invalid; ordinary error order applies instead of DAG admission.
    Invalid,
}

/// Prepared metadata belongs to the request; workers only borrow this immutable value.
#[derive(Debug)]
pub struct RequestOperators {
    /// Original-plan shape metadata owned by this exact immutable request, never file state:
    /// one slot per rule, compiled when the request graph first resolves that rule's regions.
    pub ordered_existence:
        Vec<std::sync::OnceLock<crate::domain::self_join_recogniser::types::Program>>,
    /// The prepared operator contracts of each rule, one per compiled operation.
    pub rules: Vec<Vec<Arc<crate::domain::operator::contract::types::OperatorContract>>>,
    /// How many contracts were prepared.
    pub preparations: crate::domain::operator::contract::types::PreparationCount,
}
