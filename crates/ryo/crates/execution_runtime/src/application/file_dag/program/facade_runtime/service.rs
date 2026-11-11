//! Opening one publication scope and naming the physical files of one project request.
use crate::application::file_dag::executor::service::PrimitiveExecutor;
use crate::application::file_dag::program::admission::service::AvailableInput;
use crate::application::file_dag::program::admission::types::RequiredPartitions;
use crate::application::file_dag::program::facade_runtime::types::PresenceGate;
use crate::application::file_dag::program::facade_runtime::types::Truth;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::request_identity::service::RequestIdentity;
use crate::application::file_dag::program::runtime::service::ProgramExecution;
use ryo_execution_domain::domain::coverage::types::{Covered, RuleIndex};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_execution_store::application::file_dag::service::open_program_owner;
use ryo_values::domain::runs::types::Skipped;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) fn publication_owner(
    program: &Arc<PreparedProgram>,
    services: &ProgramServices,
    digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
) -> Result<PublicationScope, OperatingError> {
    // The project phase opens no route scope: its owner records into an empty profile.
    let owner = open_program_owner(
        ryo_execution_store::application::file_dag::relations::types::ProducerCapacity(
            program.program().regions().len(),
        ),
        services.derived().clone(),
        ReuseEligibility::Eligible,
        ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile::default(),
    );
    // It reads every file's frontier rows and is never prepared for one file: it holds no graph cache.
    owner.derived().serve_project()?;
    Ok(PublicationScope {
        request: Arc::clone(&services.request),
        program: Arc::clone(program.program()),
        owner,
        execution: std::cell::OnceCell::new(),
        walk:
            crate::application::file_dag::program::native_walk::service::project::service::ProjectWalk::new(),
        populations: std::cell::RefCell::new(Default::default()),
        symbols: std::cell::OnceCell::new(),
        decodes: crate::application::file_dag::frontiers::decodes::service::FrontierDecodes::new(
            digest,
        ),
    })
}

/// The finite physical files named by this project request. No absent frame is
/// synthesized from a missing index, and a skipped file is never complete-empty.
pub(crate) fn required_partitions(
    program: &Arc<PreparedProgram>,
    rule: RuleIndex,
    files: &[ryo_values::domain::files::types::CorpusInput],
    covered: &[Covered],
    skipped: &[Skipped],
    inapplicable: &[ryo_relations::domain::relations::types::FileId],
    refused: &[ryo_relations::domain::relations::types::FileId],
    services: &ProgramServices,
) -> Result<RequiredPartitions, OperatingError> {
    program.rule_metadata(rule)?;
    RequiredPartitions::from_request(
        &services.request,
        files,
        covered,
        skipped,
        inapplicable,
        refused,
    )
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
    pub(super) scratch: ryo_execution_domain::domain::table_scratch::types::LazyScratch<
        ryo_execution_domain::domain::table_scratch::types::SelectScratch,
    >,
}

pub(crate) struct ProgramServices {
    pub(super) request: Arc<RequestIdentity>,
    pub(super) kernels: Arc<dyn PrimitiveExecutor + Send>,
    pub(super) derived:
        ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess,
    /// The observer the request's caller asked to be told of the engine's work, if any.
    pub(super) observer: Option<Arc<dyn crate::outbound::observer::interface::ObserverPort>>,
}

impl ProgramServices {
    pub(crate) fn new(
        kernels: Arc<dyn PrimitiveExecutor + Send>,
        derived: impl Into<
            ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess,
        >,
        corpus: &[ryo_values::domain::files::types::CorpusInput],
    ) -> Self {
        Self {
            kernels,
            derived: derived.into(),
            request: Arc::new(RequestIdentity::new(corpus)),
            observer: None,
        }
    }
    /// The same services, telling `observer` of the engine's work.
    pub(crate) fn observed(
        mut self,
        observer: Option<Arc<dyn crate::outbound::observer::interface::ObserverPort>>,
    ) -> Self {
        self.observer = observer;
        self
    }
    /// The observer the request's caller asked to be told of the engine's work, if any.
    pub(crate) fn observer(
        &self,
    ) -> Option<&dyn crate::outbound::observer::interface::ObserverPort> {
        self.observer.as_deref()
    }
    pub(crate) fn request(&self) -> &Arc<RequestIdentity> {
        &self.request
    }
    pub(crate) fn kernels(&self) -> &dyn PrimitiveExecutor {
        self.kernels.as_ref()
    }
    pub(crate) fn derived(
        &self,
    ) -> &ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess {
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
    pub(super) walk: crate::application::file_dag::program::native_walk::service::project::service::ProjectWalk,
    pub(super) populations: std::cell::RefCell<
        crate::application::file_dag::program::project_population::service::Populations<
            crate::application::file_dag::program::project_population::types::ProjectFrontierPopulation<AvailableInput>,
            crate::application::file_dag::program::project_population::types::ProjectInvocationKey<
                crate::application::file_dag::program::restored::types::ValidatedFrontier,
            >,
            ryo_execution_store::application::file_dag::derived::population::service::PopulationId,
        >,
    >,
    pub(super) symbols: std::cell::OnceCell<ryo_language::domain::symbols::types::TransportPermit>,
    /// The scope's frontier decodes: the packed words of one population are decoded once.
    pub(super) decodes: crate::application::file_dag::frontiers::decodes::service::FrontierDecodes,
}

impl PublicationScope {
    #[cfg(any(test, feature = "fixtures"))]
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

    #[cfg(any(test, feature = "fixtures"))]
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
                self.owner.release();
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
