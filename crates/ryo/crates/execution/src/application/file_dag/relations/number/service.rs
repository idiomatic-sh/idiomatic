//! Numeric admission witness minted only by the current physical population owner.
use crate::application::file_dag::derived::population::types::PopulationId;
use crate::application::file_dag::relations::handle::service::PublishedInput;
use crate::domain::executable_program::types::RegionId;
use crate::domain::ordered_existence::types::NumberRequirement;

/// The borrowed publication pins its exact population/generation for the invocation.
/// Construction is private: a schema or a caller assertion cannot mint this witness.
pub(in crate::application::file_dag) struct AdmittedNumber<'a> {
    pub(in crate::application::file_dag::relations) publication: &'a PublishedInput,
    _producer: RegionId,
    _population: PopulationId,
    pub(in crate::application::file_dag) column: ryo_values::domain::plan_parts::types::ColumnIndex,
}

impl AdmittedNumber<'_> {
    pub(in crate::application::file_dag) fn publication(&self) -> &PublishedInput {
        self.publication
    }
}

#[cfg(test)]
mod test;

mod issuance;
