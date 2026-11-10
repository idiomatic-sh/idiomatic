//! What a project phase offers a caller.
use crate::application::file_dag::program::project_phase::types::{
    OccurrenceNumber, PhasePartials, PhaseRules, PhaseStop,
};
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use crate::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::stored_facts::types::ArenaBytes;
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
/// rules through `evaluate_rules` are the one phase call.
/// ```compile_fail,E0603
/// use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectTask;
/// ```
/// The interpreter's counters are not the seam's: a caller cannot read its readiness counters,
/// ```compile_fail,E0599
/// use ryo_execution_runtime::application::file_dag::program::project_phase::{
///     interface::{ProjectRootExecution, ProjectStage},
///     types::ProjectPhase,
/// };
/// fn readiness(project: &ProjectPhase<'_>) {
///     let _ = project.readiness_counts();
/// }
/// ```
/// nor its state counters.
/// ```compile_fail,E0599
/// use ryo_execution_runtime::application::file_dag::program::project_phase::{
///     interface::{ProjectRootExecution, ProjectStage},
///     types::ProjectPhase,
/// };
/// fn state(project: &ProjectPhase<'_>) {
///     let _ = project.state_counts();
/// }
/// ```
pub trait ProjectRootExecution {
    /// One occurrence of each of `phase.rules`, in one call: each occurrence scoped and its frontiers prepared for its diagnostic
    /// once `before` admits it, the prepared ones evaluated with the `partials` they see under `guard`, and each outcome handed to
    /// `complete` with the phase's arenas and no reserved worker, in declaration order. A failure of one occurrence stays with
    /// it; occurrences after the guard's refusal are absent.
    fn evaluate_rules<H, T, E>(
        &self,
        phase: PhaseRules<'_>,
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
    ) -> ProjectBatch<Result<T, PhaseStop<E>>, E>;
}
pub trait ProjectStage: ProjectRootExecution {
    fn finish(self) -> Result<(), OperatingError>;
}
