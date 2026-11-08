//! What a request execution offers a caller.
use crate::application::file_dag::frontiers::errors::PreparationError;
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::program::file_session::types::CaptureContext;
#[cfg(any(test, feature = "fixtures"))]
use crate::application::file_dag::program::file_session::types::FileInput;
use crate::application::file_dag::program::file_session::types::FileRules;
use crate::application::file_dag::program::file_session::types::FileRunInput;
#[cfg(any(test, feature = "fixtures"))]
use crate::application::file_dag::program::file_session::types::FileSession;
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::application::file_dag::program::project_phase::types::FrontierRequest;
use crate::application::file_dag::program::project_phase::types::{OccurrenceNumber, OrdinaryPhase, OrdinaryRules, PhasePartials};
use crate::application::file_dag::program::rule_outcome::types::{ProjectOutputOutcome, RuleOutcome};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
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
    /// Opens one file's session for a caller that drives its rules itself: the engine's own tests and harnesses only.
    /// Outside the engine a file runs through `run_file`, its one per-file route.
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
    /// Runs the ordinary project rules of this request in one phase: each declared occurrence scoped and its frontiers prepared once
    /// `before` admits it, the prepared ones evaluated with the `partials` they see and `guard` asked before each cohort, each
    /// outcome handed to `complete` in declaration order, and the phase released. A failure of one occurrence stays with it; only
    /// the phase's opening refuses the whole call.
    ///
    /// # Errors
    ///
    /// The engine's refusal to open the phase.
    fn ordinary_phase<H, T, E>(
        &self,
        phase: OrdinaryPhase<'_>,
        symbols: &mut Symbols,
        before: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
        complete: impl FnMut(OccurrenceNumber, RuleOutcome<ProjectOutputOutcome>, &mut Symbols) -> Result<T, E>,
    ) -> Result<OrdinaryRules<T, E>, OperatingError>;
    /// One mandatory owner for all ordinary project rules in this request phase.
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError>;
}
