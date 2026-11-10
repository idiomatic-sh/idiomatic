//! What a request execution offers a caller.
use crate::application::file_dag::program::file_session::types::CaptureContext;
#[cfg(any(test, feature = "fixtures"))]
use crate::application::file_dag::program::file_session::types::FileInput;
use crate::application::file_dag::program::file_session::types::FileRules;
use crate::application::file_dag::program::file_session::types::FileRunInput;
#[cfg(any(test, feature = "fixtures"))]
use crate::application::file_dag::program::file_session::types::FileSession;
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::outbound::facts::interface::FactsWorker;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
pub trait ExecutionRequest {
    /// The observation stage of the request: its project phase and the cross-file publications.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] when the request declares no observations, and the refusal of opening its project
    /// phase.
    fn observations<'a>(
        &'a self,
        arenas: &'a mut Vec<ryo_values::domain::stored_facts::types::ArenaBytes>,
    ) -> Result<ObservationStage<'a>, OperatingError>;
    fn finish(self)
    where
        Self: Sized;
    /// Opens one file's session for a caller that drives its rules itself: the engine's own tests and harnesses only.
    /// Outside the engine a file runs through `run_file`, its one per-file route.
    ///
    /// # Errors
    ///
    /// The refusal of opening the file's session: its source, intents or partials do not fit the program.
    #[cfg(any(test, feature = "fixtures"))]
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
    ///
    /// # Errors
    ///
    /// The refusal of building the phase's publication owner.
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError>;
}
