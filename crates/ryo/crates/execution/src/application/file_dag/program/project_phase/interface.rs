//! What a project phase offers a caller.
use crate::application::file_dag::program::project_phase::types::ProjectScope;
use crate::application::file_dag::program::project_phase::types::ProjectTask;
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use crate::application::file_dag::program::rule_outcome::types::ProjectRuleOutcome;
use crate::domain::coverage::types::Covered;
use crate::domain::coverage::types::InapplicableFiles;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operating::errors::OperatingError;
use crate::domain::readiness::types::InvocationStateCounts;
use crate::domain::readiness::types::ReadinessCounts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::runs::types::Skipped;
/// Project rules are collected only; no row callback delivery exists on this route.
/// ```compile_fail,E0599
/// use ryo_execution::application::file_dag::program::file_session::interface::FileExecution;
/// use ryo_execution::application::file_dag::program::project_phase::{
///     interface::{ProjectRootExecution, ProjectStage},
///     types::ProjectPhase,
/// };
/// fn deliver(project: &mut ProjectPhase<'_>) {
///     project.emit_rows();
/// }
/// ```
/// The authorized route, declared roots through the phase, compiles.
/// ```
/// use ryo_execution::application::file_dag::program::project_phase::{
///     interface::ProjectRootExecution,
///     types::{ProjectPhase, ProjectTask},
/// };
/// use ryo_execution::application::file_dag::program::rule_outcome::types::{ProjectBatch, ProjectRuleOutcome};
/// use ryo_execution::inbound::use_cases::execution::interface::{OperatingError, RuleIndex};
/// use ryo_language::domain::symbols::types::Symbols;
/// fn evaluate(
///     phase: &ProjectPhase<'_>,
///     tasks: Vec<ProjectTask<'_>>,
///     symbols: &mut Symbols,
/// ) -> ProjectBatch<Result<ProjectRuleOutcome, OperatingError>, OperatingError> {
///     phase.evaluate(tasks, symbols, |_: RuleIndex| Ok(()))
/// }
/// ```
pub trait ProjectRootExecution {
    /// Bind all actual tasks, prepare compatible root cohorts, then deliver in declaration order.
    fn evaluate<E>(
        &self,
        tasks: Vec<ProjectTask<'_>>,
        symbols: &mut Symbols,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
    ) -> ProjectBatch<Result<ProjectRuleOutcome, OperatingError>, E>;
    fn readiness_counts(&self) -> ReadinessCounts;
}
pub trait ProjectStage: ProjectRootExecution {
    fn state_counts(&self) -> InvocationStateCounts;
    fn project_scope(
        &self,
        rule: RuleIndex,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        inapplicable: &InapplicableFiles,
    ) -> Result<ProjectScope, OperatingError>;
    fn finish(self) -> Result<(), OperatingError>;
}
