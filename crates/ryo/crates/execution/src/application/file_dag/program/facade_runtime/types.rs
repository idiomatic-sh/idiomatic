//! Data of one execution's shared services, its publication scope and its file roots.
use crate::application::file_dag::executor::PrimitiveExecutor;
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::ApplicabilityProof;
use crate::application::file_dag::program::admission::AvailableInput;
use crate::application::file_dag::program::admission::RequiredPartitions;
use crate::application::file_dag::program::request_identity::RequestIdentity;
use crate::application::file_dag::program::runtime::service::ProgramExecution;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::operating::errors::OperatingError;
use crate::outbound::derived::interface::DerivedPort;
use std::{rc::Rc, sync::Arc};

/// Whether the frontier a file root publishes has been committed by its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FrontierCommit {
    Pending,
    Committed,
}
/// Whether the resident source of a file may serve a later request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReuseEligibility {
    Eligible,
    Ineligible,
}
/// Whether the native walk of a file has bound its roots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WalkPreparation {
    Pending,
    Prepared,
}
/// Whether the sources a named root reads are free of blocking coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NamedReadiness {
    Ready,
    Blocked,
}
/// A named observation output of one rule, prepared for admission.
#[derive(Clone, Copy, Debug)]
pub(super) struct PreparedNamed {
    pub(super) rule: crate::domain::coverage::types::RuleIndex,
    pub(super) output: ryo_values::domain::observations::types::ObservationIndex,
    pub(super) readiness: NamedReadiness,
}
/// The delivery, observation and capture a file declared for one rule.
#[derive(Clone, Copy, Debug)]
pub(super) struct DeclaredIntent {
    pub(super) diagnostic: crate::application::file_dag::program::protocol::types::DiagnosticDelivery,
    pub(super) observations: crate::application::file_dag::program::protocol::types::ObservationDemand,
    pub(super) capture: crate::application::file_dag::program::protocol::types::CaptureIntent,
}
/// The slot of a bound observation among the observation regions of a program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ObservationKey(pub(super) usize);
/// Where an ordinary project frontier sits among those a run was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct OrdinaryPlace(pub(super) usize);
/// Whether a run already holds the input a source names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Provision {
    Provided,
    Absent,
}
/// A project source admitted once for the run, with the source it answers.
pub(super) struct CachedInput {
    pub(super) source: crate::domain::executable_program::types::InputSource,
    pub(super) input: AvailableInput,
}

pub(super) struct FileRoot {
    pub(super) readiness: Option<ryo_values::domain::observations::types::ProducerReadiness>,
    pub(super) commit: FrontierCommit,
    pub(super) publication: Option<AdmittedInput>,
    pub(super) skipped: Option<ApplicabilityProof>,
}

pub(crate) struct ProgramServices {
    pub(super) request: Arc<RequestIdentity>,
    pub(super) kernels: Arc<dyn PrimitiveExecutor + Send>,
    pub(super) derived: Arc<dyn DerivedPort>,
}
impl ProgramServices {
    pub(crate) fn new(
        kernels: Arc<dyn PrimitiveExecutor + Send>,
        derived: Arc<dyn DerivedPort>,
        corpus: &[ryo_values::domain::files::types::CorpusInput],
    ) -> Self {
        Self {
            kernels,
            derived,
            request: Arc::new(RequestIdentity::new(corpus)),
        }
    }
    pub(in crate::application::file_dag::program) fn request(&self) -> &Arc<RequestIdentity> {
        &self.request
    }
    pub(in crate::application::file_dag::program) fn kernels(&self) -> &dyn PrimitiveExecutor {
        self.kernels.as_ref()
    }
    pub(in crate::application::file_dag::program) fn derived(&self) -> &Arc<dyn DerivedPort> {
        &self.derived
    }
}

pub(crate) struct ProjectFrontiers<'a> {
    pub publication: &'a PublicationScope,
    pub ordinary: crate::application::file_dag::frontiers::types::PreparedFrontiers,
    pub bound_inputs: &'a [crate::application::file_dag::frontiers::types::BoundInput],
}

/// A named project phase and its bound inputs must use the very same owner.
pub(crate) struct PublicationScope {
    pub(super) request: Arc<RequestIdentity>,
    pub(super) program: Arc<crate::domain::executable_program::types::ExecutableProgram>,
    pub(super) owner: Rc<RelationOwner>,
    pub(super) execution: std::cell::OnceCell<ProgramExecution>,
    pub(super) populations: std::cell::RefCell<
        crate::application::file_dag::program::project_population::service::Populations<
            crate::application::file_dag::program::project_population::types::ProjectFrontierPopulation<AvailableInput>,
            crate::application::file_dag::program::project_population::types::ProjectInvocationKey<
                crate::application::file_dag::program::restored::types::ValidatedFrontier,
            >,
            crate::application::file_dag::derived::population::types::PopulationId,
        >,
    >,
    pub(super) symbols: std::cell::OnceCell<ryo_language::domain::symbols::types::TransportPermit>,
}
impl PublicationScope {
    pub(crate) fn readiness_counts(&self) -> crate::domain::readiness::types::ReadinessCounts {
        self.execution
            .get()
            .map_or_else(Default::default, ProgramExecution::readiness_counts)
    }

    pub(crate) fn state_counts(&self) -> crate::domain::readiness::types::InvocationStateCounts {
        self.execution
            .get()
            .map_or_else(Default::default, |execution| execution.state_counts())
    }
    pub(crate) fn finish(self) -> Result<(), OperatingError> {
        match self.execution.into_inner() {
            Some(execution) => execution.finish(),
            None => {
                self.owner.release_population();
                Ok(())
            }
        }
    }
    pub(in crate::application::file_dag::program) fn program(
        &self,
    ) -> &Arc<crate::domain::executable_program::types::ExecutableProgram> {
        &self.program
    }
    pub(in crate::application::file_dag) fn owner(&self) -> &Rc<RelationOwner> {
        &self.owner
    }
    pub(in crate::application::file_dag::program) fn validate_partitions(
        &self,
        required: &RequiredPartitions,
    ) -> Result<(), OperatingError> {
        if self.request.as_ref() == required.request.as_ref() {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(in crate::application::file_dag::program) fn validate_request(
        &self,
        services: &ProgramServices,
        required: &RequiredPartitions,
    ) -> Result<(), OperatingError> {
        if self.request.as_ref() != services.request.as_ref()
            || self.request.as_ref() != required.request.as_ref()
        {
            return Err(OperatingError::NotRead);
        }
        Ok(())
    }
    pub(in crate::application::file_dag::program) fn validate_query(
        &self,
        symbols: &ryo_language::domain::symbols::types::QuerySymbols<'_>,
    ) -> Result<(), OperatingError> {
        let expected = self.symbols.get().ok_or(OperatingError::NotRead)?;
        if ryo_language::domain::symbols::service::query_has_transport_generation(symbols, expected)
        {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(super) fn claim_symbols(
        &self,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<ryo_language::domain::symbols::types::TransportPermit, OperatingError> {
        use ryo_language::domain::symbols::service::{same_transport_generation, transport_permit};
        let current = transport_permit(symbols);
        if let Some(expected) = self.symbols.get() {
            if !same_transport_generation(expected, &current) {
                return Err(OperatingError::NotRead);
            }
        } else {
            self.symbols
                .set(transport_permit(symbols))
                .map_err(|_| OperatingError::NotRead)?;
        }
        Ok(current)
    }
}
