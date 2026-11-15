//! What a request execution offers a caller.
use crate::application::file_dag::program::file_session::types::CaptureContext;
use crate::application::file_dag::program::file_session::types::FileRules;
use crate::application::file_dag::program::file_session::types::FileRunInput;
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::outbound::facts::interface::FactsWorker;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort;
/// fn request(execution: &impl ProgramRequestExecutionPort) {
///     let _ = execution;
/// }
/// ```
pub trait ProgramRequestExecutionPort {
    /// The observation stage of the request: its project phase and the cross-file publications.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] when the request declares no observations, and the refusal of opening its project
    /// phase.
    fn observations<'a>(
        &'a self,
        arenas: &'a mut crate::application::file_dag::program::request_execution::types::ObservationArenas,
    ) -> Result<ObservationStage<'a>, OperatingError>;
    fn finish(self)
    where
        Self: Sized;
    /// Runs every rule of one file in rule order in one session: first the file's sources write its rows into `facts` and are
    /// admitted, which settles the file's partiality; then each rule's emitted root, its frontiers captured, kept in `facts` and
    /// committed, its counts, then the session finished.
    fn run_file(
        &self,
        input: FileRunInput<'_>,
        symbols: &mut Symbols,
        capture: CaptureContext<'_>,
        facts: &mut dyn FactsWorker,
    ) -> FileRules;
    /// One mandatory owner for all ordinary project rules in this request phase.
    ///
    /// # Errors
    ///
    /// The refusal of building the phase's publication owner.
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError>;
}
