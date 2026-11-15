//! A request execution: its files, its project phase and its observation stage.
use crate::application::file_dag::frontiers::errors::FrontiersError;
use crate::application::file_dag::frontiers::service::PreparationPolicy;
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::program::facade_runtime;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::file_session::types::CaptureContext;
use crate::application::file_dag::program::file_session::types::FileRules;
use crate::application::file_dag::program::file_session::types::FileRunInput;
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::application::file_dag::program::project_phase::types::FrontierDemand;
use crate::application::file_dag::program::project_phase::types::FrontierRequest;
use crate::application::file_dag::program::project_phase::types::ProjectInput;
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::outbound::facts::interface::FactsWorker;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::derived::relation_owner::types::ScopePartials;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::CorpusInput;
use std::sync::Arc;
impl ProgramRequestExecutionPort for RequestExecution {
    fn observations<'a>(
        &'a self,
        arenas: &'a mut crate::application::file_dag::program::request_execution::types::ObservationArenas,
    ) -> Result<ObservationStage<'a>, OperatingError> {
        crate::application::file_dag::file_pass::service::served(&self.pass)?;
        let metadata = self
            .program
            .observation_metadata()
            .ok_or(OperatingError::NotRun)?;
        let phase = self.project_phase()?;
        let publications =
            crate::application::file_dag::observations::types::CrossPublications::new(
                metadata,
                &phase.publication,
                &mut arenas.0,
            );
        Ok(ObservationStage {
            phase,
            publications,
        })
    }
    fn finish(self) {
        drop(self);
    }

    fn run_file(
        &self,
        input: FileRunInput<'_>,
        symbols: &mut Symbols,
        capture: CaptureContext<'_>,
        facts: &mut dyn FactsWorker,
    ) -> FileRules {
        let request = crate::application::file_dag::file_pass::types::PassRequest {
            program: &self.program,
            admission: if input.intents.len() != self.program.rule_count().0 {
                Err(OperatingError::ShapeMismatch)
            } else if self.services.request().contains(input.file)
                == crate::application::file_dag::program::request_identity::types::CorpusMembership::Unlisted
            {
                Err(OperatingError::NotRead)
            } else {
                Ok(())
            },
            paths: self.services.request().paths(),
            observer: self.services.observer(),
        };
        crate::application::file_dag::file_pass::service::rules::service::run_rules(
            request, &self.pass, input, symbols, capture, facts,
        )
    }
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError> {
        crate::application::file_dag::file_pass::service::served(&self.pass)?;
        Ok(ProjectPhase {
            request: self,
            publication: std::rc::Rc::new(facade_runtime::service::publication_owner(
                &self.program,
                &self.services,
                Arc::clone(&self.digest),
            )?),
        })
    }
}
impl RequestExecution {
    /// Prepares the frontiers of one occurrence over its scope. Crate-private: the phase's `evaluate_occurrences` is its one
    /// caller, so no caller outside the engine acquires or holds a project population.
    pub(crate) fn prepare_frontiers(
        &self,
        input: FrontierRequest<'_>,
        symbols: &mut Symbols,
    ) -> Result<PreparedFrontiers, FrontiersError> {
        let policy = match input.demand {
            FrontierDemand::Diagnostic => PreparationPolicy::Diagnostic,
            FrontierDemand::Observations { ready } => {
                PreparationPolicy::Observations { ready: &ready.0 }
            }
        };
        self.frontiers.prepared(
            &self.program,
            input.rule,
            input.first,
            input.facts,
            input.project,
            input.project_symbols,
            symbols,
            &input.scope.required,
            policy,
        )
    }
    /// Sessions exist only inside the crate-private `ProjectPhase::evaluate`, one per declared root, which only
    /// `ProjectRootExecution::evaluate_occurrences` calls.
    pub(crate) fn open_project<'a>(
        &'a self,
        input: ProjectInput<'a>,
        symbols: &mut Symbols,
    ) -> Result<facade_runtime::project_execution::service::ProjectRun<'a>, OperatingError> {
        let frontiers = facade_runtime::service::ProjectFrontiers {
            publication: &input.scope.publication,
            ordinary: input.ordinary,
            bound_inputs: input.bound,
        };
        let inner = facade_runtime::project_execution::service::open_project(
            &self.program,
            input.scope.required.clone(),
            ScopePartials::Project {
                evaluation: input.evaluation,
                publication: input.publication,
            },
            frontiers,
            symbols,
            input.arenas,
            Arc::clone(&self.services),
        )?;
        Ok(inner)
    }
}

/// A request begins over the prepared program's kernels and shared services.
pub fn begin(
    program: &PreparedExecutionProgram,
    corpus: &[CorpusInput],
    observer: Option<Arc<dyn crate::outbound::observer::interface::ObserverPort>>,
) -> RequestExecution {
    RequestExecution {
        program: Arc::clone(&program.parts.inner),
        services: Arc::new(
            ProgramServices::new(
                Arc::clone(&program.parts.kernels),
                program.parts.derived.clone(),
                corpus,
            )
            .observed(observer),
        ),
        frontiers: crate::application::file_dag::frontiers::cache::service::FrontierCache::new(),
        digest: Arc::clone(&program.parts.digest),
        pass: Arc::clone(&program.parts.pass),
    }
}
