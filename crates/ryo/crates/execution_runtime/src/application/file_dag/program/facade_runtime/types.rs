//! Data of one execution's shared services, its publication scope and its file roots.
use crate::application::file_dag::executor::PrimitiveExecutor;
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::ApplicabilityProof;
use crate::application::file_dag::program::admission::AvailableInput;
use crate::application::file_dag::program::admission::RequiredPartitions;
use crate::application::file_dag::program::request_identity::RequestIdentity;
use crate::application::file_dag::program::runtime::service::ProgramExecution;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::relations::RelationOwner;
use std::{rc::Rc, sync::Arc};

/// Whether the frontier a file root publishes has been committed by its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FrontierCommit {
    Pending,
    Committed,
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
    pub(super) rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
    pub(super) output: ryo_values::domain::records::types::ObservationIndex,
    pub(super) readiness: NamedReadiness,
}
/// The delivery, observation and capture a file declared for one rule.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DeclaredIntent {
    pub(super) delivery: ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery,
    pub(super) demand: ryo_execution_domain::domain::rule_intent::types::ObservationDemand,
    pub(super) capture_intent: ryo_execution_domain::domain::rule_intent::types::CaptureIntent,
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
    pub(super) source: ryo_execution_domain::domain::executable_program::types::InputSource,
    pub(super) input: AvailableInput,
}

/// What one presence test or formula answered on the current file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Truth {
    Unknown,
    Holds,
    Fails,
}
/// Whether a file decides presence: off on any file with a partial read, where every root runs in full.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PresenceGate {
    On,
    Off,
}
/// A run rule's prepared root, which the presence gate may find absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RootCandidate {
    pub(super) rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
    pub(super) region: ryo_execution_domain::domain::executable_program::types::RegionId,
}
/// Whether a file instantiated a region: a slot, only a dormant reader count, or nothing at all.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Instantiated {
    Never,
    Slot,
    Dormant,
}
/// Whether a region's inputs are released, or still owed by it.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputRelease {
    Released,
    Owed,
}
/// What a file left of one region, observed only by tests.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RegionProbe {
    pub(crate) instantiated: Instantiated,
    pub(crate) inputs: Option<InputRelease>,
}
/// What a file binds when its first entry point reads it: its selected readers and the roots presence may find absent.
pub(crate) struct PendingActivation {
    pub(super) readers: Vec<ryo_execution_domain::domain::executable_program::types::RegionId>,
    pub(super) candidates: Vec<RootCandidate>,
    pub(super) gate: PresenceGate,
}
/// One file's presence memo: what each test and formula of the program answered on the file, filled lazily, each
/// relation's pending tests decided together in one pass over its rows.
pub(crate) struct FilePresence {
    /// Off once a read of the file's rows fails: no further row is read, and every formula holds.
    pub(super) gate: PresenceGate,
    pub(super) tests: Vec<Truth>,
    pub(super) formulas: Vec<Truth>,
    /// The relations the file has read, by presence or by a probe.
    pub(super) read: Vec<ryo_relations::domain::relations::types::Relation>,
    /// Room for one relation row, the tests still open on a relation, and a comparison memo, kept for the file.
    pub(super) cells: Vec<ryo_relations::domain::relations::types::Value>,
    pub(super) open: Vec<ryo_execution_domain::domain::executable_program::presence::types::TestId>,
    pub(super) memo: Vec<ryo_execution_domain::domain::operating::types::Memo>,
    pub(super) scratch: ryo_execution_domain::domain::table_scratch::types::SelectScratch,
}
pub(super) struct FileRoot {
    pub(super) readiness: Option<ryo_values::domain::records::types::ProducerReadiness>,
    pub(super) commit: FrontierCommit,
    pub(super) publication: Option<AdmittedInput>,
    pub(super) skipped: Option<ApplicabilityProof>,
}

pub(crate) struct ProgramServices {
    pub(super) request: Arc<RequestIdentity>,
    pub(super) kernels: Arc<dyn PrimitiveExecutor + Send>,
    pub(super) derived:
        ryo_execution_store::application::file_dag::derived::owner::types::DerivedAccess,
}
impl ProgramServices {
    pub(crate) fn new(
        kernels: Arc<dyn PrimitiveExecutor + Send>,
        derived: impl Into<
            ryo_execution_store::application::file_dag::derived::owner::types::DerivedAccess,
        >,
        corpus: &[ryo_values::domain::files::types::CorpusInput],
    ) -> Self {
        Self {
            kernels,
            derived: derived.into(),
            request: Arc::new(RequestIdentity::new(corpus)),
        }
    }
    pub(crate) fn request(&self) -> &Arc<RequestIdentity> {
        &self.request
    }
    pub(crate) fn kernels(&self) -> &dyn PrimitiveExecutor {
        self.kernels.as_ref()
    }
    pub(crate) fn derived(
        &self,
    ) -> &ryo_execution_store::application::file_dag::derived::owner::types::DerivedAccess {
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
    pub(super) program: Arc<ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram>,
    pub(super) owner: Rc<RelationOwner>,
    pub(super) execution: std::cell::OnceCell<ProgramExecution>,
    /// The scope's project-stage walk: every cohort of the scope leases from its batches.
    pub(super) walk: crate::application::file_dag::program::native_walk::project::service::ProjectWalk,
    pub(super) populations: std::cell::RefCell<
        crate::application::file_dag::program::project_population::service::Populations<
            crate::application::file_dag::program::project_population::types::ProjectFrontierPopulation<AvailableInput>,
            crate::application::file_dag::program::project_population::types::ProjectInvocationKey<
                crate::application::file_dag::program::restored::types::ValidatedFrontier,
            >,
            ryo_execution_store::application::file_dag::derived::population::types::PopulationId,
        >,
    >,
    pub(super) symbols: std::cell::OnceCell<ryo_language::domain::symbols::types::TransportPermit>,
    /// The scope's frontier decodes: the packed words of one population are decoded once.
    pub(super) decodes: crate::application::file_dag::frontiers::types::FrontierDecodes,
}
impl PublicationScope {
    pub(crate) fn readiness_counts(
        &self,
    ) -> ryo_execution_domain::domain::readiness::types::ReadinessCounts {
        self.execution
            .get()
            .map_or_else(Default::default, ProgramExecution::readiness_counts)
    }

    /// The project-stage walk's totals, retired batches included.
    #[cfg(test)]
    pub(crate) fn walk_counts(
        &self,
    ) -> ryo_execution_domain::domain::shared_walk::types::WalkCounts {
        self.walk.counts()
    }

    /// How many packed frontiers the scope decoded.
    #[cfg(test)]
    pub(crate) fn frontier_decodes(
        &self,
    ) -> crate::application::file_dag::frontiers::types::DecodeCount {
        self.decodes.count()
    }

    pub(crate) fn state_counts(
        &self,
    ) -> ryo_execution_domain::domain::readiness::types::InvocationStateCounts {
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
    pub(crate) fn program(
        &self,
    ) -> &Arc<ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram>{
        &self.program
    }
    pub(crate) fn owner(&self) -> &Rc<RelationOwner> {
        &self.owner
    }
    pub(crate) fn validate_partitions(
        &self,
        required: &RequiredPartitions,
    ) -> Result<(), OperatingError> {
        if self.request.as_ref() == required.request.as_ref() {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(crate) fn validate_request(
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
    pub(crate) fn validate_query(
        &self,
        symbols: &ryo_language::domain::symbols::types::QuerySymbols<'_>,
    ) -> Result<(), OperatingError> {
        let expected = self.symbols.get().ok_or(OperatingError::NotRead)?;
        if ryo_language::domain::symbols::service::query_has_transport_generation(symbols, expected)
            == ryo_language::domain::symbols::types::Generation::Same
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
            if same_transport_generation(expected, &current)
                == ryo_language::domain::symbols::types::Generation::Different
            {
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
