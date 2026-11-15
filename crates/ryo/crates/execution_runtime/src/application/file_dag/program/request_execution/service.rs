//! A request execution: its files, its project phase and its observation stage.
use crate::application::file_dag::program::file_session::types::CaptureContext;
use crate::application::file_dag::program::file_session::types::FileRules;
use crate::application::file_dag::program::file_session::types::FileRunInput;
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort;
use crate::application::file_dag::program::request_execution::types::ProgramServices;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::application::file_dag::program::request_identity::service::RequestIdentity;
use crate::outbound::facts::interface::FactsWorker;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::CorpusInput;
use std::sync::Arc;
impl ProgramRequestExecutionPort for RequestExecution {
    fn observations<'a>(
        &'a self,
        arenas: &'a mut crate::application::file_dag::program::request_execution::types::ObservationArenas,
    ) -> Result<ObservationStage<'a>, OperatingError> {
        let metadata = self
            .program
            .observation_metadata()
            .ok_or(OperatingError::NotRun)?;
        let phase = self.project_phase()?;
        Ok(ObservationStage {
            phase,
            publications: crate::application::file_dag::publications::types::Publications::new(
                metadata,
                &mut arenas.0,
            ),
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
        // The file's facts are opened here, so the pass reads them only through the lookup it is handed.
        let FileRunInput {
            language: _,
            file,
            intents,
            source,
            sources,
            active,
            said,
            recovery,
            answering,
            semantic,
            kinds: _,
            first,
        } = input;
        // What the request's last file left, its demand refilled: the file's sources are asked only for what its running rules
        // read.
        let mut scratch = crate::application::file_dag::file_pass::service::taken(&self.pass);
        crate::application::file_dag::file_pass::service::file_demand(
            &self.program,
            intents,
            &mut scratch.demand,
        );
        let lookup = crate::application::file_dag::fact_lookup::service::opened(
            sources,
            facts,
            crate::application::file_dag::fact_lookup::types::FileWants {
                active,
                said,
                recovery,
            },
            symbols,
            &scratch.demand,
        );
        crate::application::file_dag::file_pass::service::rules::service::run_rules(
            request,
            &self.pass,
            crate::application::file_dag::file_pass::types::OpenedFile {
                file,
                intents,
                source,
                answering,
                semantic,
                first,
            },
            lookup,
            symbols,
            capture,
            scratch,
        )
    }
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError> {
        Ok(ProjectPhase {
            request: self,
            declared: std::cell::RefCell::default(),
        })
    }
}
/// A request begins over the prepared program and the request's files.
pub fn begin(
    program: &PreparedExecutionProgram,
    corpus: &[CorpusInput],
    observer: Option<Arc<dyn crate::outbound::observer::interface::ObserverPort>>,
) -> RequestExecution {
    RequestExecution {
        program: Arc::clone(&program.parts.inner),
        services: Arc::new(ProgramServices::new(corpus).observed(observer)),
        pass: Arc::clone(&program.parts.pass),
    }
}

impl ProgramServices {
    pub(crate) fn new(corpus: &[CorpusInput]) -> Self {
        Self {
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
}
