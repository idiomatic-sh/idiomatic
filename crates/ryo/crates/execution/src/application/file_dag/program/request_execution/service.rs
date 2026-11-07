//! A request execution: its files, its project phase and its observation stage.
use crate::application::file_dag::frontiers::errors::PreparationError;
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::program::facade_runtime;
use crate::application::file_dag::program::file_session::types::FileInput;
use crate::application::file_dag::program::file_session::types::FileSession;
use crate::application::file_dag::program::file_session::types::SourceLanguage;
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::project_phase::types::ProjectInput;
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::application::file_dag::program::request_execution::interface::ExecutionRequest;
use crate::application::file_dag::program::project_phase::types::FrontierDemand;
use crate::application::file_dag::program::project_phase::types::FrontierRequest;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::application::file_dag::scope::ScopePartials;
use crate::domain::operating::errors::OperatingError;
use crate::outbound::facts::types::ResidentStability;
use ryo_language::domain::symbols::types::Symbols;
use std::sync::Arc;
use crate::application::file_dag::program::facade_runtime::types::ProgramServices;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_values::domain::files::types::CorpusInput;
impl ExecutionRequest for RequestExecution {
    fn observations<'a>(
        &'a self,
        arenas: &'a mut Vec<ryo_values::domain::stored_facts::types::ArenaBytes>,
    ) -> Result<ObservationStage<'a>, OperatingError> {
        let metadata = self
            .program
            .observation_metadata()
            .ok_or(OperatingError::NotRun)?;
        let phase = self.project_phase()?;
        let publications =
            crate::application::file_dag::observations::types::CrossPublications::new(
                metadata,
                &phase.publication,
                arenas,
            );
        Ok(ObservationStage {
            phase,
            publications,
        })
    }
    fn prepare_frontiers(
        &self,
        input: FrontierRequest<'_>,
        symbols: &mut Symbols,
    ) -> Result<PreparedFrontiers, PreparationError> {
        let readiness = match input.demand {
            FrontierDemand::Diagnostic => Vec::new(),
            FrontierDemand::Observations { ready } => ready
                .iter()
                .map(|ready| {
                    if *ready {
                        crate::application::file_dag::frontiers::types::InputReadiness::Ready
                    } else {
                        crate::application::file_dag::frontiers::types::InputReadiness::Unavailable
                    }
                })
                .collect(),
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
            match input.demand {
                FrontierDemand::Diagnostic => {
                    crate::application::file_dag::frontiers::service::PreparationPolicy::Diagnostic
                }
                FrontierDemand::Observations { .. } => {
                    crate::application::file_dag::frontiers::service::PreparationPolicy::Observations {
                        ready: &readiness,
                    }
                }
            },
        )
    }
    fn finish(self) {
        drop(self);
    }

    fn open_file<'a, 'facts: 'a>(
        &'a self,
        input: FileInput<'a, 'facts>,
        symbols: &mut Symbols,
    ) -> Result<FileSession<'a, 'facts>, OperatingError> {
        let mut vocabulary = input.kinds;
        let vocabulary = crate::domain::kind_sets::service::prepare_vocabulary(
            &mut vocabulary,
            input.elements,
            symbols,
            ryo_values::domain::files::types::SourceView(input.source),
        );
        let inner = facade_runtime::file_execution::open_file(
            &self.program,
            input.file,
            ScopePartials::File {
                evaluation: input.evaluation,
                publication: input.publication,
            },
            input.intents,
            Arc::clone(&self.services),
            ryo_values::domain::files::types::SourceView(input.source),
            if matches!(input.language, SourceLanguage::Recognized)
                && input.resident_stability == ResidentStability::Stable
            {
                crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible
            } else {
                crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Ineligible
            },
            symbols,
            input.semantic,
            vocabulary,
        )?;
        Ok(FileSession {
            inner,
            program: Arc::clone(&self.program),
            read: input.read,
            elements: input.elements,
            file: input.file,
        })
    }
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError> {
        Ok(ProjectPhase {
            request: self,
            publication: std::rc::Rc::new(facade_runtime::service::publication_owner(
                &self.program,
                &self.services,
            )?),
        })
    }
}
impl RequestExecution {
    /// Sessions exist only inside `ProjectRootExecution::evaluate`, one per declared root.
    pub(in crate::application::file_dag::program) fn open_project<'a>(
        &'a self,
        input: ProjectInput<'a>,
        symbols: &mut Symbols,
    ) -> Result<facade_runtime::project_execution::ProjectRun<'a>, OperatingError> {
        let frontiers =
            crate::application::file_dag::program::facade_runtime::types::ProjectFrontiers {
                publication: &input.scope.publication,
                ordinary: input.ordinary,
                bound_inputs: input.bound,
            };
        let inner = facade_runtime::project_execution::open_project(
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
pub(crate) fn begin(program: &PreparedExecutionProgram, corpus: &[CorpusInput]) -> RequestExecution {
    RequestExecution {
        program: Arc::clone(&program.inner),
        services: Arc::new(ProgramServices::new(
            Arc::clone(&program.kernels),
            program.derived.clone(),
            corpus,
        )),
        frontiers: crate::application::file_dag::frontiers::types::FrontierCache::new(),
    }
}
