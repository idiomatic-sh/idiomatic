//! Validation of admitted protocol handovers; no scheduling or dependency acquisition.
use super::types::ObservedPublication;
use crate::domain::operating::errors::OperatingError;

impl ObservedPublication {
    pub(in crate::application::file_dag::program) fn from_admitted(
        admitted: crate::application::file_dag::program::admission::AdmittedInput,
    ) -> Self {
        Self { admitted }
    }
    pub(in crate::application::file_dag::program) fn admitted(
        &self,
    ) -> &crate::application::file_dag::program::admission::AdmittedInput {
        &self.admitted
    }
    pub(in crate::application::file_dag) fn validate_transfer_readiness(
        &self,
        owner: &std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        readiness: &ryo_values::domain::observations::types::ProducerReadiness,
    ) -> Result<(), OperatingError> {
        self.admitted.publication(owner)?;
        match (self.admitted.coverage()?, readiness) {
            (
                crate::domain::completion::types::CoverageState::Complete,
                ryo_values::domain::observations::types::ProducerReadiness::Complete,
            ) => Ok(()),
            (
                crate::domain::completion::types::CoverageState::Partial { files },
                ryo_values::domain::observations::types::ProducerReadiness::Unknown(_),
            ) if !files.is_empty() => Ok(()),
            (
                crate::domain::completion::types::CoverageState::Inapplicable { files },
                ryo_values::domain::observations::types::ProducerReadiness::Unknown(_),
            ) if !files.is_empty() => Ok(()),
            _ => Err(OperatingError::ObservationIncomplete {
                reason:
                    crate::application::file_dag::program::constants::PACKED_READINESS_DISAGREES
                        .into(),
            }),
        }
    }
}
