//! Validation of admitted protocol handovers; no scheduling or dependency acquisition.
use super::types::ObservedPublication;
use ryo_execution_domain::domain::operating::errors::OperatingError;

impl ObservedPublication {
    pub(crate) fn from_admitted(
        admitted: crate::application::file_dag::program::admission::AdmittedInput,
    ) -> Self {
        Self { admitted }
    }
    pub(crate) fn admitted(
        &self,
    ) -> &crate::application::file_dag::program::admission::AdmittedInput {
        &self.admitted
    }
    pub(crate) fn validate_transfer_readiness(
        &self,
        owner: &std::rc::Rc<ryo_execution_store::application::file_dag::relations::RelationOwner>,
        readiness: &ryo_values::domain::records::types::ProducerReadiness,
    ) -> Result<(), OperatingError> {
        self.admitted.publication(owner)?;
        match (self.admitted.coverage(), readiness) {
            (
                ryo_execution_domain::domain::completion::types::CoverageState::Complete,
                ryo_values::domain::records::types::ProducerReadiness::Complete,
            ) => Ok(()),
            (
                ryo_execution_domain::domain::completion::types::CoverageState::Partial {
                    files,
                    ..
                },
                ryo_values::domain::records::types::ProducerReadiness::Unknown(_),
            ) if !files.is_empty() => Ok(()),
            (
                ryo_execution_domain::domain::completion::types::CoverageState::Inapplicable {
                    files,
                },
                ryo_values::domain::records::types::ProducerReadiness::Unknown(_),
            ) if !files.is_empty() => Ok(()),
            _ => Err(OperatingError::ObservationIncomplete {
                reason:
                    crate::application::file_dag::program::constants::PACKED_READINESS_DISAGREES
                        .into(),
            }),
        }
    }
}
