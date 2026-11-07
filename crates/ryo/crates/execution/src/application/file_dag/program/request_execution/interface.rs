//! What a request execution offers a caller.
use crate::application::file_dag::frontiers::errors::PreparationError;
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::program::file_session::types::CaptureContext;
use crate::application::file_dag::program::file_session::types::FileInput;
use crate::application::file_dag::program::file_session::types::FileRules;
use crate::application::file_dag::program::file_session::types::FileRunInput;
use crate::application::file_dag::program::file_session::types::FileSession;
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::application::file_dag::program::project_phase::types::FrontierRequest;
use crate::domain::operating::errors::OperatingError;
use crate::outbound::facts::interface::FactsWorker;
use ryo_language::domain::symbols::types::Symbols;
pub trait ExecutionRequest {
    fn observations<'a>(
        &'a self,
        arenas: &'a mut Vec<ryo_values::domain::stored_facts::types::ArenaBytes>,
    ) -> Result<ObservationStage<'a>, OperatingError>;
    fn prepare_frontiers(
        &self,
        input: FrontierRequest<'_>,
        symbols: &mut Symbols,
    ) -> Result<PreparedFrontiers, PreparationError>;
    fn finish(self)
    where
        Self: Sized;
    fn open_file<'a, 'facts: 'a>(
        &'a self,
        input: FileInput<'a, 'facts>,
        symbols: &mut Symbols,
    ) -> Result<FileSession<'a, 'facts>, OperatingError>;
    /// Runs every rule of one file in rule order in one session: each rule's emitted root, its frontiers captured, kept in
    /// `facts` and committed, its counts, then the session finished. The rows the rules read are the worker's `facts`.
    fn run_file(
        &self,
        input: FileRunInput<'_>,
        symbols: &mut Symbols,
        capture: CaptureContext<'_>,
        facts: &dyn FactsWorker,
    ) -> FileRules;
    /// One mandatory owner for all ordinary project rules in this request phase.
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError>;
}
