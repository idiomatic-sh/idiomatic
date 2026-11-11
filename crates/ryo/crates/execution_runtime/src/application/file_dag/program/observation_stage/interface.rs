//! What an observation stage offers a caller: its declared batches run behind one call.
use crate::application::file_dag::program::observation_stage::types::{
    Install, ObservationBatches,
};
use crate::application::file_dag::program::project_phase::types::{
    OccurrenceNumber, PhasePartials, PhaseStop,
};
use crate::application::file_dag::program::rule_outcome::types::{
    ProjectOutputOutcome, RuleOutcome,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::stored_facts::types::ArenaBytes;
/// The stage runs its own protocol: a caller cannot name a publication slot,
/// ```compile_fail,E0603
/// use ryo_execution_runtime::application::file_dag::frontiers::types::PublicationSlot;
/// ```
/// nor reserve one by hand,
/// ```compile_fail,E0624
/// use ryo_execution_runtime::application::file_dag::program::observation_stage::service::ObservationStage;
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// fn reserve(stage: &mut ObservationStage<'_>) {
///     let _slot = stage.reserve(RuleIndex(0));
/// }
/// ```
/// nor evaluate its occurrences unbound, past the bindings `run_batches` makes.
/// ```compile_fail,E0599
/// use ryo_execution_runtime::application::file_dag::program::observation_stage::service::ObservationStage;
/// use ryo_execution_runtime::application::file_dag::program::project_phase::interface::ProjectRootExecution;
/// use ryo_execution_runtime::application::file_dag::program::project_phase::types::{PhasePartials, PhaseRules};
/// use ryo_execution_domain::domain::operating::errors::OperatingError;
/// use ryo_language::domain::symbols::types::Symbols;
/// fn bypass(stage: &ObservationStage<'_>, phase: PhaseRules<'_>) {
///     let _ = stage.evaluate_rules(
///         phase,
///         &mut Symbols::default(),
///         |_| Ok::<(), OperatingError>(()),
///         |_| -> Result<PhasePartials, OperatingError> { Err(OperatingError::NotRun) },
///         |_| Ok(()),
///         |_, _, _, _, _| Ok(()),
///     );
/// }
/// ```
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::observation_stage::interface::ProgramObservationStagePort;
/// fn stage(stage: &mut impl ProgramObservationStagePort) {
///     let _ = stage;
/// }
/// ```
pub trait ProgramObservationStagePort {
    /// Runs every declared occurrence of the stage's batches: the rules no occurrence names finished, the frontiers restored, then
    /// per batch each occurrence bound (its frontiers' readiness, its publication slot, its inputs) in declaration order, the bound
    /// ones evaluated with `before`, `partials` and `guard` and each outcome handed to `complete` with the batch's arenas and the
    /// worker its slot reserved, then each occurrence handed to `deliver` in declaration order, what it returns installed in its
    /// slot and its rule finished. Every callback is given the occurrence's place in `batches.occurrences`. The first error of a
    /// batch ends the stage.
    ///
    /// # Errors
    ///
    /// The first error of the stage: a rule it did not declare, a refused restoration, delivery, install or finish, or the guard's
    /// refusal.
    fn run_batches<H, T, E: From<OperatingError>>(
        self,
        batches: ObservationBatches<'_>,
        symbols: &mut Symbols,
        before: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
        complete: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            &[ArenaBytes],
            Option<WorkerNumber>,
            &mut Symbols,
        ) -> Result<T, E>,
        deliver: impl FnMut(OccurrenceNumber, Result<T, PhaseStop<E>>) -> Result<Option<Install>, E>,
    ) -> Result<(), E>
    where
        Self: Sized;
}
