//! Portable named-output contracts. Application execution owns readiness, publication and lifetime.
use crate::domain::plan_parts::types::Input;
use crate::domain::plans::types::StepScope;
use crate::domain::runs::types::RuleId;
use ryo_relations::domain::relations::types::CellKind;

/// An explicit output name written by the query author.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationName(pub Box<str>);
/// A local bound input name written by the consumer author.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationBindingName(pub Box<str>);
/// A bound input's stable local position within one compiled consumer plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationBindingId(pub u32);
/// A declared output's local position within one producer plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationIndex(pub u32);
/// A semantic column name in a named-output contract.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationColumnName(pub Box<str>);
/// One ordered column in a named-output contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationColumn {
    /// Semantic column spelling, independent of any physical row representation.
    pub name: ObservationColumnName,
    /// Actual cell kind.
    pub kind: CellKind,
}
/// An authored root retained independently of the diagnostic Emit root.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanObservation {
    /// Public output name.
    pub name: ObservationName,
    /// The actual typed producer root.
    pub source: Input,
    /// Inferred ordered schema.
    pub columns: Vec<ObservationColumn>,
    /// Inferred execution phase.
    pub scope: StepScope,
}
/// An explicit producer source. Missing execution is not a successful empty result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationProducer {
    /// A real selected query's explicitly authored export.
    Output {
        /// Original configured rule identity, never an execution-order position.
        rule: RuleId,
        /// Explicit producer export.
        output: ObservationName,
    },
    /// The configured selection contains this rule but no executable query is registered.
    /// Request validation rejects this declaration if the rule actually has an executable plan.
    NoQuery {
        /// Original configured rule identity.
        rule: RuleId,
    },
}
/// The schema of a consumer's bound input, supplied during query compilation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundObservationInput {
    /// Local source name in the consumer query.
    pub name: ObservationBindingName,
    /// Required ordered columns.
    pub columns: Vec<ObservationColumn>,
    /// Phase inferred from the actual producers.
    pub scope: StepScope,
}
/// One explicit rule dependency, resolved to real producer roots by the common request graph.
/// Input rows are the distinct set union of all complete producer outputs, including singleton bindings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationBinding {
    /// Local source name in the consumer query.
    pub name: ObservationBindingName,
    /// Expected ordered schema.
    pub columns: Vec<ObservationColumn>,
    /// Explicit outputs; an empty list is not secretly populated from unrelated diagnostics.
    pub producers: Vec<ObservationProducer>,
}
/// Why a declared producer cannot supply an observed result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationReason(pub Box<str>);
/// Readiness belongs to an actually admitted output, distinct from selected-rule inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProducerReadiness {
    /// The producer root completed, including a real zero-row result.
    Complete,
    /// A dependency, mapping or retention refusal prevents a complete result.
    Unknown(ObservationReason),
    /// The producer is not applicable in this file/scope.
    Skipped,
    /// The configured producer has no executable query; it is never treated as complete-empty.
    NoQuery,
}

/// Canonical bytes of one named-input declaration set, distinct from any presentation string.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObservationIdentityBytes(pub Vec<u8>);

/// Version of persisted root-readiness evidence, independent of row content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationReadinessVersion(pub u32);

/// Number of existing frontier buckets assigned to one selected rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationFrontierCount(pub u32);
/// One selected rule's position and actual retained bucket count in a named request.
#[derive(Clone, Debug, PartialEq)]
pub struct ObservationRuleLayout {
    /// Original configured rule identity in this ordered request.
    pub rule: RuleId,
    /// Number of ordinary retained frontier buckets, before observation status slots.
    pub frontiers: ObservationFrontierCount,
    /// Only actual persistent producer roots contribute a query/config definition.
    pub outputs: ObservationOutputLayout,
}

/// Semantic identity of ordered producer/output definitions plus query/config/schema identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationReadinessLayout(pub crate::domain::stored_facts::types::Digest);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// Position within one validated semantic layout; not an independently stable identity.
pub struct ObservationReadinessSlot(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Number of logical status slots in the current semantic layout.
pub struct ObservationReadinessCount(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Contiguous status range belonging to one prepared producer rule.
pub struct ObservationReadinessRange {
    /// First slot within the validated map.
    pub start: ObservationReadinessSlot,
    /// Number of slots borrowed by this producer.
    pub length: ObservationReadinessCount,
}
/// Only the common pure validation boundary can lend this checked view.
#[derive(Clone, Copy, Debug)]
pub struct ValidatedFrontiers<'a>(pub(super) &'a crate::domain::records::types::FrontierReadiness);

#[derive(Clone, Debug, PartialEq)]
/// Whether a rule defines persistent publication readiness in this request.
pub enum ObservationOutputLayout {
    /// This rule still reserves its ordinary bucket range but defines no persisted observation bits.
    NoPersistentRoots,
    /// Actual retained outputs whose meaning must match persisted completion bits.
    Persistent {
        /// Stable query/helper/parameter/source-selection/catalog/project identity, never a process address.
        configuration: crate::domain::stored_facts::types::Digest,
        /// Semantic names, ordered schema and phase; transient step IDs are not encoded as identity.
        outputs: Vec<PlanObservation>,
    },
}
