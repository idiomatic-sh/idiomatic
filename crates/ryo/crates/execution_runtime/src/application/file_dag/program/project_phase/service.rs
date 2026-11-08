//! A project phase: roots evaluated in declaration order over the request.
use crate::application::file_dag::program::facade_runtime;
use crate::application::file_dag::program::project_phase::interface::ProjectRootExecution;
use crate::application::file_dag::program::project_phase::interface::ProjectStage;
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::application::file_dag::program::project_phase::types::PhaseScope;
use crate::application::file_dag::program::project_phase::types::ProjectTask;
use crate::application::file_dag::program::project_phase::types::FrontierDemand;
use crate::application::file_dag::program::project_phase::types::FrontierRequest;
use crate::application::file_dag::program::project_phase::types::OccurrenceNumber;
use crate::application::file_dag::program::project_phase::types::OrdinaryPhase;
use crate::application::file_dag::program::project_phase::types::OrdinaryRules;
use crate::application::file_dag::program::project_phase::types::PhasePartials;
use crate::application::file_dag::program::project_phase::types::PhaseStop;
use crate::application::file_dag::program::project_phase::types::PreparedOccurrence;
use crate::application::file_dag::program::project_phase::types::ReadyOccurrence;
use crate::application::file_dag::program::project_phase::types::ProjectInput;
use crate::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use ryo_execution_domain::domain::rule_intent::types::{EmitDemand, ObservationDemand, RuleDemand};
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use crate::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use ryo_execution_domain::domain::coverage::types::Covered;
use ryo_execution_domain::domain::coverage::types::InapplicableFiles;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::readiness::types::InvocationStateCounts;
use ryo_execution_domain::domain::readiness::types::ReadinessCounts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::runs::types::Skipped;
impl ProjectRootExecution for ProjectPhase<'_> {
    fn evaluate<E>(
        &self,
        tasks: Vec<ProjectTask<'_>>,
        symbols: &mut Symbols,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
    ) -> ProjectBatch<Result<RuleOutcome<ProjectOutputOutcome>, OperatingError>, E> {
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
impl RequestExecution {
    /// One ordinary project phase in one call: the phase opened, each declared occurrence prepared in declaration order once
    /// `before` admits it, the prepared ones evaluated as one batch under `guard`, each outcome completed in declaration order, and
    /// the phase released once nothing of it is held.
    pub(crate) fn ordinary<H, T, E>(
        &self,
        phase: OrdinaryPhase<'_>,
        symbols: &mut Symbols,
        mut before: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        mut partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
        mut complete: impl FnMut(OccurrenceNumber, RuleOutcome<ProjectOutputOutcome>, &mut Symbols) -> Result<T, E>,
    ) -> Result<OrdinaryRules<T, E>, OperatingError> {
        let stage = self.project_phase()?;
        let prepared = phase
            .occurrences
            .iter()
            .copied()
            .enumerate()
            .map(|(at, rule)| {
                let occurrence = OccurrenceNumber(at);
                // What `before` returns is held until the occurrence's frontiers are prepared.
                let _held = before(occurrence).map_err(PhaseStop::Caller)?;
                self.prepare_ordinary(&stage, &phase, rule, symbols)
            })
            .collect::<Vec<_>>();
        // The frontiers of the occurrences that reach evaluation, in declaration order; the scopes and partials stay with their
        // occurrence for the tasks to borrow.
        let mut frontiers = Vec::with_capacity(prepared.len());
        let entries = prepared
            .into_iter()
            .enumerate()
            .map(|(at, prepared)| {
                let PreparedOccurrence { scope, frontier } = prepared?;
                let partials = partials(OccurrenceNumber(at)).map_err(PhaseStop::Caller)?;
                frontiers.push(frontier);
                Ok(ReadyOccurrence { scope, partials })
            })
            .collect::<Vec<Result<ReadyOccurrence, PhaseStop<E>>>>();
        let mut frontiers = frontiers.into_iter();
        let tasks = entries
            .iter()
            .zip(phase.occurrences)
            .filter_map(|(entry, rule)| {
                let Ok(ready) = entry else {
                    return None;
                };
                Some(ProjectTask {
                    rule: *rule,
                    demand: RuleDemand {
                        emit: EmitDemand::Enabled,
                        observations: ObservationDemand::Required,
                    },
                    input: ProjectInput {
                        scope: &ready.scope,
                        evaluation: &ready.partials.evaluation,
                        publication: &ready.partials.publication,
                        ordinary: frontiers.next()?,
                        bound: &[],
                        arenas: phase.arenas,
                    },
                })
            })
            .collect();
        let (evaluated, refused) = match stage.evaluate(tasks, symbols, guard) {
            ProjectBatch::Complete(completed) => (completed, None),
            ProjectBatch::Refused { completed, error } => (completed, Some(error)),
        };
        let occurrences = {
            let mut evaluated = evaluated.into_iter();
            let mut occurrences = Vec::with_capacity(entries.len());
            for (at, entry) in entries.into_iter().enumerate() {
                let result = match entry {
                    Err(stop) => Err(stop),
                    Ok(_) => {
                        let Some(outcome) = evaluated.next() else {
                            if refused.is_some() {
                                break;
                            }
                            occurrences.push(Err(PhaseStop::Unevaluated));
                            continue;
                        };
                        outcome.map_err(PhaseStop::Operating).and_then(|outcome| {
                            complete(OccurrenceNumber(at), outcome, symbols).map_err(PhaseStop::Caller)
                        })
                    }
                };
                occurrences.push(result);
            }
            occurrences
        };
        // Every scope and outcome of the phase is released above, so the phase owns its publication alone again.
        let finished = stage.finish();
        Ok(OrdinaryRules {
            occurrences,
            refused,
            finished,
        })
    }

    /// One admitted occurrence of an ordinary phase: scoped over the files its rule applies to, and its frontiers prepared for the
    /// diagnostic from its rule's first bucket.
    fn prepare_ordinary<E>(
        &self,
        stage: &ProjectPhase<'_>,
        phase: &OrdinaryPhase<'_>,
        rule: RuleIndex,
        symbols: &mut Symbols,
    ) -> Result<PreparedOccurrence, PhaseStop<E>> {
        let first = *phase.first.get(rule.0).ok_or(PhaseStop::MissingFirstBucket)?;
        let inapplicable = phase
            .inapplicable
            .get(rule.0)
            .ok_or(PhaseStop::Operating(OperatingError::NotRun))?;
        let scope = stage
            .project_scope(rule, phase.corpus, phase.covered, phase.skipped, inapplicable)
            .map_err(PhaseStop::Operating)?;
        let frontier = self
            .prepare_frontiers(
                FrontierRequest {
                    scope: &scope,
                    rule,
                    first,
                    facts: phase.facts,
                    project: phase.project,
                    project_symbols: phase.project_symbols,
                    demand: FrontierDemand::Diagnostic,
                },
                symbols,
            )
            .map_err(PhaseStop::Frontiers)?;
        Ok(PreparedOccurrence { scope, frontier })
    }
}
