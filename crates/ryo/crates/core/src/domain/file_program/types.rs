//! Immutable request graph and bounded, invocation-local shared results.

use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NodeInput {
    Relation(Relation),
    Node(NodeId),
    /// A named input reads already-published producer roots, never demands them under this rule.
    Observation(ObservationGraphInputId),
}

#[derive(Debug)]
pub(crate) struct Node {
    pub contract: Arc<crate::domain::operator::contract::types::OperatorContract>,
    pub physical:
        crate::domain::table_model::collections::representations::publication::PreparedPhysical,
    /// Executable registered operation with normalized local input numbers.
    pub operation: ryo_values::domain::plans::types::Operation,
    /// Ordered typed dependencies retained for the central executor.
    pub inputs: Vec<NodeInput>,
    /// Invalid synthetic plans keep ordinary error order instead of DAG admission.
    pub schema_valid: bool,
    /// Transitive physical facts required before the application admits this node.
    pub relations: Vec<Relation>,
}

#[derive(Debug)]
pub(crate) struct RequestGraph {
    /// Reviewed alternatives indexed by their replaced Select producer.
    pub ordered_existence: Vec<Option<crate::domain::ordered_existence::types::ResolvedAlternative>>,
    /// Named roots extend this request graph; ordinary requests retain no observation allocation.
    pub observations: Option<GraphObservations>,
    /// Producers of declared derived inputs, computed once for the whole request.
    pub derived_sources: Vec<bool>,
    pub plans: Vec<usize>,
    pub nodes: Vec<Node>,
    /// Only internal kept cuts may own a file-local result. Other entries are None.
    pub cuts: Vec<Vec<Option<NodeId>>>,
    /// Unique eligible nodes consumed by each rule, prepared once per request.
    pub consumers: Vec<Vec<NodeId>>,
    /// Graph outputs consumed by residual operations, in original step order.
    pub boundaries: Vec<Vec<(usize, NodeId)>>,
    /// Parent-before-child rule-local live-edge counts, compiled once per request.
    pub rule_liveness: Vec<Vec<(NodeId, usize)>>,
}

/// One physical file generation; no table escapes this value or enters a global cache.
#[derive(Debug)]
pub(crate) struct FileProgram {
    pub graph: Arc<RequestGraph>,
    pub remaining: Vec<usize>,
    pub entries: Vec<(NodeId, Arc<Table>)>,
    pub shared_executions: u64,
    pub peak_retained_cells: usize,
}

use ryo_values::domain::rows::types::Table;
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
        output: ryo_values::domain::observations::types::ObservationIndex,
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
    pub input: ryo_values::domain::observations::types::ObservationBindingId,
    /// The actual inferred schema and phase.
    pub schema: ryo_values::domain::observations::types::BoundObservationSchema,
    /// Actual typed producers; no runtime name lookup remains.
    pub producers: Vec<ResolvedObservationProducer>,
}
/// Compile-time descriptors and their resolved graph references travel together.
pub(crate) struct PreparedObservationInputs {
    /// Compiler declarations in original local input order.
    pub schemas: Vec<ryo_values::domain::observations::types::BoundObservationSchema>,
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
        layout: ryo_values::domain::observations::types::ObservationReadinessLayout,
        order: &'a [crate::domain::coverage::types::RuleIndex],
        batches: &'a [Vec<crate::domain::coverage::types::RuleIndex>],
        bindings: &'a [Vec<ResolvedObservationBinding>],
    },
}

/// Stable local bound input qualified by its original consumer index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ObservationSlot {
    pub rule: usize,
    pub input: ryo_values::domain::observations::types::ObservationBindingId,
}
/// Actual prepared root location; unsupported admission never becomes complete-empty.
#[derive(Clone, Debug)]
pub(crate) enum GraphObservationRoot {
    PerFile(NodeId),
    CrossFile,
    Unavailable,
}
/// One named root and its actual final-consumer accounting.
#[derive(Clone, Debug)]
pub(crate) struct GraphObservationOutput {
    pub root: GraphObservationRoot,
    pub consumers: Vec<usize>,
}
/// The existing request graph owns both typed binding edges and root lifetime declarations.
#[derive(Debug)]
pub(crate) struct GraphObservations {
    pub layout: ryo_values::domain::observations::types::ObservationReadinessLayout,
    /// Request-qualified bound slots; graph edges carry a compact ID rather than widening every ordinary edge.
    pub slots: Vec<ObservationSlot>,
    /// Logical output status slots in stable rule/output order; aliases may share a physical bucket.
    pub frontiers: Vec<ObservedFrontierRoot>,
    /// Prepared outgoing retirement edges, indexed by original consumer rule.
    pub retire: Vec<Vec<ObservationRetirement>>,
    /// Producer-local constant-time root publication lookup; never rescan all exports per node.
    pub publish: Vec<
        std::collections::HashMap<
            NodeId,
            Vec<ryo_values::domain::observations::types::ObservationIndex>,
        >,
    >,
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
    pub output: ryo_values::domain::observations::types::ObservationIndex,
}

/// A persisted named root uses an existing frontier bucket, not another row store.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ObservedFrontierRoot {
    pub producer: crate::domain::coverage::types::RuleIndex,
    pub output: ryo_values::domain::observations::types::ObservationIndex,
    pub frontier: ryo_values::domain::plan_parts::types::InputNumber,
}

/// One request-wide binding slot; its payload is the same width as an ordinary NodeId edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ObservationGraphInputId(pub usize);
