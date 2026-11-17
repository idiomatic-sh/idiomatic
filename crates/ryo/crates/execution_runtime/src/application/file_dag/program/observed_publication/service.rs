//! What an occurrence after the files published of a named output: its rows and how completely they were read, handed to the
//! stage's install and to the consumers bound to it.
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::records::types::ProducerReadiness;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;

impl ObservedPublication {
    /// Checks that the readiness a transfer states for the publication agrees with how completely its rows were read.
    ///
    /// # Errors
    ///
    /// `NotRead` for a publication of another stage or of one that ended; `PACKED_READINESS_DISAGREES` when the readiness does
    /// not agree.
    pub(crate) fn validate_transfer_readiness(
        &self,
        readiness: &ProducerReadiness,
        stage: &Arc<()>,
    ) -> Result<(), OperatingError> {
        // A publication is the stage's that made it: one held past its stage is no publication of the next.
        if self.stage.upgrade().is_none()
            || !std::sync::Weak::ptr_eq(&self.stage, &Arc::downgrade(stage))
        {
            return Err(OperatingError::NotRead);
        }
        match (&self.coverage, readiness) {
            (CoverageState::Complete, ProducerReadiness::Complete) => Ok(()),
            (CoverageState::Partial { files, .. }, ProducerReadiness::Unknown(_))
                if !files.is_empty() =>
            {
                Ok(())
            }
            (CoverageState::Inapplicable { files }, ProducerReadiness::Unknown(_))
                if !files.is_empty() =>
            {
                Ok(())
            }
            _ => Err(OperatingError::ObservationIncomplete {
                reason:
                    crate::application::file_dag::program::constants::PACKED_READINESS_DISAGREES
                        .into(),
            }),
        }
    }
}

/// Opaque application handover of what an occurrence after the files published of a named output: its rows, in the stage's
/// symbols, and how completely they were read. Packed bytes may accompany it, but never substitute for it.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::observed_publication::service::ObservedPublication;
/// fn handed(publication: &ObservedPublication) -> ObservedPublication {
///     publication.clone()
/// }
/// ```
#[derive(Clone, Debug)]
pub struct ObservedPublication {
    /// The rows.
    pub(crate) rows: Arc<Table>,
    /// How completely they were read.
    pub(crate) coverage: CoverageState,
    /// The stage that made it, alive while the stage runs.
    pub(crate) stage: std::sync::Weak<()>,
}
