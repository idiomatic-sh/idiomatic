//! A project phase: roots evaluated in declaration order over the request.
use crate::application::file_dag::frontiers::types::PublicationSlot;
use crate::application::file_dag::program::facade_runtime;
use crate::application::file_dag::program::project_phase::interface::ProgramProjectPhasePort;
use crate::application::file_dag::program::project_phase::interface::ProjectRootExecution;
use crate::application::file_dag::program::project_phase::types::FrontierDemand;
use crate::application::file_dag::program::project_phase::types::FrontierRequest;
use crate::application::file_dag::program::project_phase::types::OccurrenceNumber;
use crate::application::file_dag::program::project_phase::types::PhaseOccurrence;
use crate::application::file_dag::program::project_phase::types::PhaseOccurrences;
use crate::application::file_dag::program::project_phase::types::PhasePartials;
use crate::application::file_dag::program::project_phase::types::PhaseRules;
use crate::application::file_dag::program::project_phase::types::PhaseScope;
use crate::application::file_dag::program::project_phase::types::PhaseStop;
use crate::application::file_dag::program::project_phase::types::PreparedOccurrence;
use crate::application::file_dag::program::project_phase::types::ProjectInput;
use crate::application::file_dag::program::project_phase::types::ProjectTask;
use crate::application::file_dag::program::project_phase::types::ReadyOccurrence;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use crate::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use ryo_execution_domain::domain::coverage::types::Covered;
use ryo_execution_domain::domain::coverage::types::InapplicableFiles;
use ryo_execution_domain::domain::coverage::types::RefusedFiles;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
#[cfg(any(test, feature = "fixtures"))]
use ryo_execution_domain::domain::readiness::types::InvocationStateCounts;
#[cfg(any(test, feature = "fixtures"))]
use ryo_execution_domain::domain::readiness::types::ReadinessCounts;
use ryo_execution_domain::domain::rule_intent::types::{EmitDemand, ObservationDemand, RuleDemand};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::ArenaBytes;
impl ProjectRootExecution for ProjectPhase<'_> {
    fn evaluate_rules<H, T, E>(
        &self,
        phase: PhaseRules<'_>,
        symbols: &mut Symbols,
        before: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
        mut complete: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            &[ArenaBytes],
            Option<WorkerNumber>,
            &mut Symbols,
        ) -> Result<T, E>,
    ) -> ProjectBatch<Result<T, PhaseStop<E>>, E> {
        let occurrences = phase
            .rules
            .iter()
            .map(|rule| PhaseOccurrence {
                rule: *rule,
                bound: &[],
                slot: PublicationSlot::Unused,
                demand: FrontierDemand::Diagnostic,
            })
            .collect::<Vec<_>>();
        let arenas = phase.arenas;
        self.evaluate_occurrences(
            PhaseOccurrences {
                occurrences: &occurrences,
                first: phase.first,
                corpus: phase.corpus,
                covered: phase.covered,
                skipped: phase.skipped,
                inapplicable: phase.inapplicable,
                refused: phase.refused,
                facts: phase.facts,
                project: phase.project,
                project_symbols: phase.project_symbols,
                arenas,
            },
            symbols,
            before,
            partials,
            guard,
            |occurrence, outcome, slot, symbols: &mut Symbols| {
                complete(occurrence, outcome, arenas, reserved(slot), symbols)
            },
        )
    }
}
/// The worker a publication slot reserved, when it reserved one.
pub(crate) fn reserved(slot: PublicationSlot) -> Option<WorkerNumber> {
    match slot {
        PublicationSlot::Unused => None,
        PublicationSlot::Reserved(worker) => Some(worker),
    }
}
impl ProgramProjectPhasePort for ProjectPhase<'_> {
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
impl ProjectPhase<'_> {
    /// One batch of declared occurrences in one call, the ordinary phase's and each observation batch's: each occurrence scoped
    /// and its frontiers prepared for its demand once `before` admits it, the prepared ones evaluated with their bound inputs and
    /// the `partials` they see under `guard`, and each outcome handed to `complete` with its publication slot, in declaration
    /// order. A failure of one occurrence stays with it; occurrences after the guard's refusal are absent.
    pub(crate) fn evaluate_occurrences<H, T, E>(
        &self,
        phase: PhaseOccurrences<'_>,
        symbols: &mut Symbols,
        mut before: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        mut partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
        mut complete: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            PublicationSlot,
            &mut Symbols,
        ) -> Result<T, E>,
    ) -> ProjectBatch<Result<T, PhaseStop<E>>, E> {
        let prepared = phase
            .occurrences
            .iter()
            .enumerate()
            .map(|(at, occurrence)| {
                // What `before` returns is held until the occurrence's frontiers are prepared.
                let _held = before(OccurrenceNumber(at)).map_err(PhaseStop::Caller)?;
                self.prepare_occurrence(&phase, occurrence, symbols)
            })
            .collect::<Vec<_>>();
        // The frontiers of the occurrences that reach evaluation, in declaration order; the scopes, partials and bound inputs stay
        // with their occurrence for the tasks to borrow.
        let mut frontiers = Vec::with_capacity(prepared.len());
        let entries = prepared
            .into_iter()
            .enumerate()
            .map(|(at, prepared)| {
                let PreparedOccurrence {
                    scope,
                    frontier,
                    bound,
                    slot,
                } = prepared?;
                let partials = partials(OccurrenceNumber(at)).map_err(PhaseStop::Caller)?;
                frontiers.push(frontier);
                Ok(ReadyOccurrence {
                    scope,
                    partials,
                    bound,
                    slot,
                })
            })
            .collect::<Vec<Result<ReadyOccurrence<'_>, PhaseStop<E>>>>();
        let mut frontiers = frontiers.into_iter();
        let tasks = entries
            .iter()
            .zip(phase.occurrences)
            .filter_map(|(entry, occurrence)| {
                let Ok(ready) = entry else {
                    return None;
                };
                Some(ProjectTask {
                    rule: occurrence.rule,
                    demand: RuleDemand {
                        emit: EmitDemand::Enabled,
                        observations: ObservationDemand::Required,
                    },
                    input: ProjectInput {
                        scope: &ready.scope,
                        evaluation: &ready.partials.evaluation,
                        publication: &ready.partials.publication,
                        ordinary: frontiers.next()?,
                        bound: ready.bound,
                        arenas: phase.arenas,
                    },
                })
            })
            .collect();
        let (evaluated, refused) = match self.evaluate(tasks, symbols, guard) {
            ProjectBatch::Complete(completed) => (completed, None),
            ProjectBatch::Refused { completed, error } => (completed, Some(error)),
        };
        let mut evaluated = evaluated.into_iter();
        let mut occurrences = Vec::with_capacity(entries.len());
        for (at, entry) in entries.into_iter().enumerate() {
            let result = match entry {
                Err(stop) => Err(stop),
                Ok(ready) => {
                    let Some(outcome) = evaluated.next() else {
                        if refused.is_some() {
                            break;
                        }
                        occurrences.push(Err(PhaseStop::Unevaluated));
                        continue;
                    };
                    outcome.map_err(PhaseStop::Operating).and_then(|outcome| {
                        complete(OccurrenceNumber(at), outcome, ready.slot, symbols)
                            .map_err(PhaseStop::Caller)
                    })
                }
            };
            occurrences.push(result);
        }
        match refused {
            Some(error) => ProjectBatch::Refused {
                completed: occurrences,
                error,
            },
            None => ProjectBatch::Complete(occurrences),
        }
    }
    /// The interpreter's readiness counters, for its own tests.
    ///
    /// # Examples
    ///
    /// ```
    /// // A project phase exists only inside a running project phase: the call's types, checked.
    /// use ryo_execution_domain::domain::readiness::types::ReadinessCounts;
    /// use ryo_execution_runtime::application::file_dag::program::project_phase::service::ProjectPhase;
    /// fn readiness(phase: &ProjectPhase<'_>) -> ReadinessCounts {
    ///     phase.fixture_readiness_counts()
    /// }
    /// ```
    #[cfg(any(test, feature = "fixtures"))]
    pub fn fixture_readiness_counts(&self) -> ReadinessCounts {
        self.publication.readiness_counts()
    }
    /// The interpreter's state counters, for its own tests.
    ///
    /// # Examples
    ///
    /// ```
    /// // A project phase exists only inside a running project phase: the call's types, checked.
    /// use ryo_execution_domain::domain::readiness::types::InvocationStateCounts;
    /// use ryo_execution_runtime::application::file_dag::program::project_phase::service::ProjectPhase;
    /// fn states(phase: &ProjectPhase<'_>) -> InvocationStateCounts {
    ///     phase.fixture_state_counts()
    /// }
    /// ```
    #[cfg(any(test, feature = "fixtures"))]
    pub fn fixture_state_counts(&self) -> InvocationStateCounts {
        self.publication.state_counts()
    }
    /// The files `rule` reads in this phase, sharing the phase's publication. Crate-private: `evaluate_occurrences` scopes each
    /// occurrence itself, so no caller outside the engine holds a scope.
    pub(crate) fn project_scope(
        &self,
        rule: RuleIndex,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        inapplicable: &InapplicableFiles,
        refused: &RefusedFiles,
    ) -> Result<PhaseScope, OperatingError> {
        let required = facade_runtime::service::required_partitions(
            &self.request.program,
            rule,
            corpus,
            covered,
            skipped,
            &inapplicable.0,
            &refused.0,
            &self.request.services,
        )?;
        Ok(PhaseScope {
            required,
            publication: std::rc::Rc::clone(&self.publication),
        })
    }
    /// Bind all actual tasks, prepare compatible root cohorts, then deliver in declaration order. Crate-private: a caller hands the
    /// phase its declared occurrences through `evaluate_occurrences`, never tasks of its own.
    pub(crate) fn evaluate<E>(
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
            roots.push(facade_runtime::project_execution::types::ProjectRoot {
                rule: task.rule,
                demand: task.demand,
                run,
            });
        }
        facade_runtime::project_execution::service::evaluate_roots(roots, symbols, guard)
    }
    /// One admitted occurrence: scoped over the files its rule applies to, and its frontiers prepared for its demand from its
    /// rule's first bucket.
    fn prepare_occurrence<'o, E>(
        &self,
        phase: &PhaseOccurrences<'_>,
        occurrence: &PhaseOccurrence<'o>,
        symbols: &mut Symbols,
    ) -> Result<PreparedOccurrence<'o>, PhaseStop<E>> {
        let rule = occurrence.rule;
        let first = *phase
            .first
            .get(rule.0)
            .ok_or(PhaseStop::MissingFirstBucket)?;
        let inapplicable = phase
            .inapplicable
            .get(rule.0)
            .ok_or(PhaseStop::Operating(OperatingError::NotRun))?;
        let refused = phase
            .refused
            .get(rule.0)
            .ok_or(PhaseStop::Operating(OperatingError::NotRun))?;
        let scope = self
            .project_scope(
                rule,
                phase.corpus,
                phase.covered,
                phase.skipped,
                inapplicable,
                refused,
            )
            .map_err(PhaseStop::Operating)?;
        let frontier = self
            .request
            .prepare_frontiers(
                FrontierRequest {
                    scope: &scope,
                    rule,
                    first,
                    facts: phase.facts,
                    project: phase.project,
                    project_symbols: phase.project_symbols,
                    demand: occurrence.demand,
                },
                symbols,
            )
            .map_err(PhaseStop::Frontiers)?;
        Ok(PreparedOccurrence {
            scope,
            frontier,
            bound: occurrence.bound,
            slot: occurrence.slot,
        })
    }
}

/// One project-phase publication/interpreter and symbol generation, shared across rules.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::project_phase::service::ProjectPhase;
/// fn phase(phase: &ProjectPhase<'_>) {
///     let _ = phase;
/// }
/// ```
pub struct ProjectPhase<'a> {
    pub(crate) request: &'a RequestExecution,
    pub(crate) publication: std::rc::Rc<
        crate::application::file_dag::program::facade_runtime::service::PublicationScope,
    >,
}
