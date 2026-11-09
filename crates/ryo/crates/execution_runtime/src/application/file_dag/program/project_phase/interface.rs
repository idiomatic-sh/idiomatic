//! What a project phase offers a caller.
use crate::application::file_dag::frontiers::types::PublicationSlot;
use crate::application::file_dag::program::project_phase::types::{
    OccurrenceNumber, PhaseOccurrences, PhasePartials, PhaseStop,
};
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use crate::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::readiness::types::InvocationStateCounts;
use ryo_execution_domain::domain::readiness::types::ReadinessCounts;
use ryo_language::domain::symbols::types::Symbols;
/// Project rules are collected only; no row callback delivery exists on this route.
/// ```compile_fail,E0599
/// use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution;
/// use ryo_execution_runtime::application::file_dag::program::project_phase::{
///     interface::{ProjectRootExecution, ProjectStage},
///     types::ProjectPhase,
/// };
/// fn deliver(project: &mut ProjectPhase<'_>) {
///     project.emit_rows();
/// }
/// ```
/// A caller cannot hand the phase tasks of its own: the root evaluation and its task are crate-private, and the declared
/// occurrences through `evaluate_occurrences` are the one phase call.
/// ```compile_fail,E0603
/// use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectTask;
/// ```
pub trait ProjectRootExecution {
    /// One batch of declared occurrences in one call, the ordinary phase's and each observation batch's: each occurrence scoped
    /// and its frontiers prepared for its demand once `before` admits it, the prepared ones evaluated with their bound inputs and
    /// the `partials` they see under `guard`, and each outcome handed to `complete` with its publication slot, in declaration
    /// order. A failure of one occurrence stays with it; occurrences after the guard's refusal are absent.
    fn evaluate_occurrences<H, T, E>(
        &self,
        phase: PhaseOccurrences<'_>,
        symbols: &mut Symbols,
        before: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
        complete: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            PublicationSlot,
            &mut Symbols,
        ) -> Result<T, E>,
    ) -> ProjectBatch<Result<T, PhaseStop<E>>, E>;
    fn readiness_counts(&self) -> ReadinessCounts;
}
pub trait ProjectStage: ProjectRootExecution {
    fn state_counts(&self) -> InvocationStateCounts;
    fn finish(self) -> Result<(), OperatingError>;
}
