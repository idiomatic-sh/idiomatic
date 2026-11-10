//! Checked declarations retain their private source provenance.
pub mod source_partitions;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Relation;

/// Number of immutable hosted relation snapshots.
#[derive(Debug)]
pub(crate) struct HostedCount(pub usize);
/// Exact declaration invariant that failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PartitionRefusal {
    BindingOrdinal,
    MissingDeclaration,
    UndeclaredInput,
    MissingConsumer,
    ConsumerMismatch,
    MissingQuery,
    MissingRule,
    MissingOutput,
    OutputMismatch,
    MissingStep,
    IndeterminatePhase,
    MissingBinding,
    PhaseMismatch,
}

/// Which partition of its producer an input must be bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PartitionRequirement {
    /// The same rows for the whole request.
    RequestInvariant,
    /// The rows of the current file.
    CurrentFile,
    /// Must be bound to the consumer's finite demanded partition set.
    Project,
    /// The partition of the input the rows descend from.
    InputLineage,
}

/// Mandatory producer declarations. No operation kind implies broadcast completeness.
/// Used by preparation/admission only; semantic kernels never receive this interface.
pub trait ProducerPartitions {
    /// The partition requirement of a host relation provided to a rule.
    ///
    /// # Errors
    ///
    /// The rule's own refusal when it is not declared, and [`OperatingError::NotRead`] for a host relation the request
    /// does not provide.
    fn hosted(
        &self,
        rule: RuleIndex,
        relation: ryo_values::domain::plan_parts::types::HostedNumber,
    ) -> Result<PartitionRequirement, OperatingError>;
    /// The partition requirement of a rule's parameter.
    ///
    /// # Errors
    ///
    /// The rule's own refusal when it is not declared, and [`OperatingError::Unbound`] for a parameter it does not
    /// bind.
    fn parameter(
        &self,
        rule: RuleIndex,
        parameter: ryo_values::domain::plan_parts::types::ParameterId,
    ) -> Result<PartitionRequirement, OperatingError>;
    /// The partition requirement of a language relation.
    fn relation(&self, relation: Relation) -> PartitionRequirement;
    /// The partition requirement of an observation.
    ///
    /// # Errors
    ///
    /// The rule's own refusal when it is not declared, and
    /// [`OperatingError::ObservationIncomplete`] for an observation it does not declare.
    fn observation(
        &self,
        rule: RuleIndex,
        input: ryo_values::domain::plan_parts::types::ObservationBindingId,
    ) -> Result<PartitionRequirement, OperatingError>;
}
