//! A project phase: roots evaluated in declaration order over the request.
use crate::application::file_dag::program::facade_runtime;
use crate::application::file_dag::program::project_phase::interface::ProjectRootExecution;
use crate::application::file_dag::program::project_phase::interface::ProjectStage;
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::application::file_dag::program::project_phase::types::PhaseScope;
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
impl ProjectRootExecution for ProjectPhase<'_> {
    fn evaluate<E>(
        &self,
        tasks: Vec<ProjectTask<'_>>,
        symbols: &mut Symbols,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
    ) -> ProjectBatch<Result<ProjectRuleOutcome, OperatingError>, E> {
        let mut roots = Vec::with_capacity(tasks.len());
        for task in tasks {
            let run = if std::rc::Rc::ptr_eq(&self.publication, &task.input.scope.publication) {
                self.request.open_project(task.input, symbols)
            } else {
                Err(OperatingError::NotRead)
            };
            roots.push(facade_runtime::project_execution::ProjectRoot {
                rule: task.rule,
                demand: task.demand,
                run,
            });
        }
        facade_runtime::project_execution::evaluate_roots(roots, symbols, guard)
    }
    fn readiness_counts(&self) -> ReadinessCounts {
        self.publication.readiness_counts()
    }
}
impl ProjectStage for ProjectPhase<'_> {
    fn state_counts(&self) -> InvocationStateCounts {
        self.publication.state_counts()
    }
    fn project_scope(
        &self,
        rule: RuleIndex,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        inapplicable: &InapplicableFiles,
    ) -> Result<PhaseScope, OperatingError> {
        let required = facade_runtime::service::required_partitions(
            &self.request.program,
            rule,
            corpus,
            covered,
            skipped,
            &inapplicable.0,
            &self.request.services,
        )?;
        Ok(PhaseScope {
            required,
            publication: std::rc::Rc::clone(&self.publication),
        })
    }
    fn finish(self) -> Result<(), OperatingError> {
        match std::rc::Rc::try_unwrap(self.publication) {
            Ok(publication) => publication.finish(),
            Err(shared) => {
                drop(shared);
                Err(OperatingError::NotRead)
            }
        }
    }
}
