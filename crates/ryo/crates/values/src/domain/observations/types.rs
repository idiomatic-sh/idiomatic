//! Portable named-output contracts. Application execution owns readiness, publication and lifetime.
use crate::domain::plan_parts::types::Input;
use crate::domain::plan_parts::types::StepScope;
use crate::domain::records::types::{ObservationReadinessCount, ObservationReadinessSlot};
use crate::domain::runs::types::RuleId;
use ryo_relations::domain::relations::types::CellKind;

/// An explicit output name written by the query author.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationName(pub Box<str>);
/// A local bound input name written by the consumer author.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationBindingName(pub Box<str>);
/// A semantic column name in a named-output contract.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationColumnName(pub Box<str>);
/// One ordered column in a named-output contract.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ObservationColumn {
    /// Semantic column spelling, independent of any physical row representation.
    pub name: ObservationColumnName,
    /// Actual cell kind.
    pub kind: CellKind,
}
/// An authored root retained independently of the diagnostic Emit root.
#[derive(Clone, Debug, PartialEq, Hash)]
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

/// Canonical bytes of one named-input declaration set, distinct from any presentation string.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObservationIdentityBytes(pub Vec<u8>);

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Contiguous status range belonging to one prepared producer rule.
pub struct ObservationReadinessRange {
    /// First slot within the validated map.
    pub start: ObservationReadinessSlot,
    /// Number of slots borrowed by this producer.
    pub length: ObservationReadinessCount,
}
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

/// What one slot of a readiness bit set holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadinessBit {
    /// The slot is marked.
    Set,
    /// The slot is not marked.
    Clear,
}

/// Text bytes written into an identity, length first.
#[derive(Clone, Copy, Debug)]
pub struct ObservationText<'a>(pub &'a [u8]);
