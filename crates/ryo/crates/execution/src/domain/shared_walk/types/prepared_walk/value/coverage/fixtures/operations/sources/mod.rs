//! The partition requirements every operation corpus plan is compiled against.
use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::types::{PartitionRequirement, ProducerPartitions};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{HostedNumber, ParameterId};
pub(crate) struct Sources;
impl ProducerPartitions for Sources {
    fn hosted(
        &self,
        _: RuleIndex,
        _: HostedNumber,
    ) -> Result<PartitionRequirement, OperatingError> {
        Ok(PartitionRequirement::RequestInvariant)
    }
    fn parameter(
        &self,
        _: RuleIndex,
        _: ParameterId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Ok(PartitionRequirement::RequestInvariant)
    }
    fn relation(&self, _: Relation) -> PartitionRequirement {
        PartitionRequirement::CurrentFile
    }
    fn observation(
        &self,
        _: RuleIndex,
        _: ryo_values::domain::observations::types::ObservationBindingId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::NotRead)
    }
}
