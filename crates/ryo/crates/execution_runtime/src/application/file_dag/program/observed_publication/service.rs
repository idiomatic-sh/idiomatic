//! Validation of admitted protocol handovers; no scheduling or dependency acquisition.
use crate::application::file_dag::program::admission::{AdmittedInput, ReadProof};
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;

impl ObservedPublication {
    /// The handover of an admitted input: its read proof, root, publication, population and generation.
    pub(crate) fn from_parts(admitted: AdmittedInput) -> Self {
        Self {
            proof: admitted.read_proof(),
            root: admitted.producer_root(),
            publication: admitted.held_publication().clone(),
            population: admitted.admitted_population(),
            generation: admitted.admitted_generation().clone(),
        }
    }
    /// The population the publication was admitted under.
    pub(crate) fn population(
        &self,
    ) -> ryo_execution_store::application::file_dag::derived::population::service::PopulationId
    {
        self.population
    }
    /// How completely the publication's rows were read.
    pub(crate) fn coverage(&self) -> &CoverageState {
        self.proof.coverage()
    }
    /// The admitted input again, over `owner`: its rows admitted under the owner's current generation.
    pub(crate) fn readmitted(
        &self,
        owner: &std::rc::Rc<
            ryo_execution_store::application::file_dag::relations::service::RelationOwner,
        >,
    ) -> Result<AdmittedInput, OperatingError> {
        let unavailable = |_| Err(OperatingError::NotRead);
        let rows = owner.rows(&self.publication, &unavailable)?;
        let admitted = owner.admit(
            &self.publication,
            ryo_execution_domain::domain::table_model::types::RepresentationInput::from(&rows),
        )??;
        AdmittedInput::from_publication(
            std::rc::Rc::clone(owner),
            self.publication.clone(),
            &admitted,
            self.root,
            ReadProof::clone(&self.proof),
        )
    }
    pub(crate) fn validate_transfer_readiness(
        &self,
        readiness: &ryo_values::domain::records::types::ProducerReadiness,
    ) -> Result<(), OperatingError> {
        self.generation.live()?;
        match (self.proof.coverage(), readiness) {
            (
                CoverageState::Complete,
                ryo_values::domain::records::types::ProducerReadiness::Complete,
            ) => Ok(()),
            (
                CoverageState::Partial { files, .. },
                ryo_values::domain::records::types::ProducerReadiness::Unknown(_),
            ) if !files.is_empty() => Ok(()),
            (
                CoverageState::Inapplicable { files },
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

/// Opaque application handover of the actual admitted population: what was read and how completely, its producer root, the
/// publication and the population and generation it was admitted under. Packed bytes may accompany it, but never substitute for
/// this evidence.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::observed_publication::service::ObservedPublication;
/// fn handed(publication: &ObservedPublication) -> ObservedPublication {
///     publication.clone()
/// }
/// ```
#[derive(Clone)]
pub struct ObservedPublication {
    pub(crate) proof: crate::application::file_dag::program::admission::ReadProof,
    pub(crate) root: crate::application::file_dag::program::admission::ProducerRoot,
    pub(crate) publication:
        ryo_execution_store::application::file_dag::relations::handle::service::Publication,
    pub(crate) population:
        ryo_execution_store::application::file_dag::derived::population::service::PopulationId,
    pub(crate) generation: ryo_execution_store::application::file_dag::relations::generation::service::ResidentGeneration,
}
