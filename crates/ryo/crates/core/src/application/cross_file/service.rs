//! Cross-file evaluation of the project batches over injected ports.
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use super::completion::service::ProjectCompletion;
use super::completion::service::collected;
use super::completion::types::CrossedWithPublications;
use super::errors::CrossFileError;
use super::constants::{MISSING_FRONTIER_BUCKET, MISSING_OUTCOME};
use super::rendering::service::Rendering;
use super::types::BatchOccurrenceBinding;
use super::types::ObservationRun;
use super::types::ProjectCrossing;
use crate::domain::answering::service::settle;
use crate::domain::answering::types::Answered;
use crate::domain::answering::types::Todo;
use crate::domain::crossing::service::inapplicable_files;
use crate::domain::crossing::service::partials_for;
use crate::domain::crossing::service::partials_for_reads;
use crate::domain::render::types::Answering;
use crate::outbound::corpus::interface::CorpusPort;
use ryo_execution::inbound::use_cases::publication::interface::PublicationUseCase;
use ryo_execution::inbound::use_cases::execution::interface::{AdmittedOutput, RuleSelection};
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ObservationExecution;
use ryo_execution::inbound::use_cases::execution::interface::ObservationPublication;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::FileObservationReadiness;
use ryo_execution::inbound::use_cases::execution::interface::FrontierDemand;
use ryo_execution::inbound::use_cases::execution::interface::OccurrenceNumber;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::PhaseOccurrence;
use ryo_execution::inbound::use_cases::execution::interface::PhaseOccurrences;
use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseStop;
use ryo_execution::inbound::use_cases::execution::interface::PreparationError;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectOutputOutcome;
use ryo_execution::inbound::use_cases::execution::interface::PublicationSlot;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::RuleOutcome;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::rule_requests::service::failed;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use std::sync::Arc;

/// Cross-file work sharing the caller's corpus resource.
pub struct CrossFile {
    /// Renders each phase's answer and packs its publications.
    pub(super) completion: ProjectCompletion,
}

impl std::fmt::Debug for CrossFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl CrossFile {
    /// Bind source access once at composition.
    #[must_use]
    pub fn new(corpus: Arc<dyn CorpusPort>, publication: Arc<dyn PublicationUseCase>) -> Self {
        Self {
            completion: ProjectCompletion::new(Rendering::new(corpus), publication),
        }
    }
    /// Completes one evaluated occurrence of a phase: its outcome collected, its answer rendered and its named roots packed into
    /// its publication slot.
    pub(super) fn complete_occurrence(
        &self,
        crossing: &ProjectCrossing<'_>,
        todo: &Todo,
        outcome: RuleOutcome<ProjectOutputOutcome>,
        slot: PublicationSlot,
        symbols: &mut Symbols,
    ) -> Result<CrossedWithPublications, CrossFileError> {
        let evaluated = collected(outcome).map_err(|error| CrossFileError::from(failed(&todo.id, &error)))?;
        self.completion
            .complete_phase(crossing, todo, evaluated, symbols, slot)
            .map_err(CrossFileError::from)
    }

    /// Execute the declared ready batches and transfer publications before their final consumer retires.
    pub(crate) fn observations(
        &self,
        run: ObservationRun<'_>,
        arenas: &mut Vec<ArenaBytes>,
        answered: &mut Answered,
        mut guard: impl FnMut(RuleIndex) -> Result<(), CrossFileError>,
    ) -> Result<(), CrossFileError> {
        let mut stage = run.services.observations(arenas)?;
        let mut selected = vec![RuleSelection::Unselected; run.domain.metadata.len()];
        for todo in run.todo {
            *selected
                .get_mut(todo.rule.0)
                .ok_or(OperatingError::NotRun)? = RuleSelection::Selected;
        }
        for (rule, selected) in selected.iter().enumerate() {
            if *selected == RuleSelection::Unselected {
                stage.finish(RuleIndex(rule))?;
            }
        }
        let readiness = run
            .domain
            .observations
            .iter()
            .map(|frame| FileObservationReadiness {
                file: frame.file,
                root: Arc::clone(&frame.root),
            })
            .collect::<Vec<_>>();
        let inapplicable = inapplicable_files(&MetadataUseCaseImpl, &run.domain);
        stage.restore_frontiers(
            run.facts,
            run.domain.files,
            run.domain.covered,
            run.domain.skipped,
            &inapplicable,
            &readiness,
            run.first_bucket,
            &selected,
        )?;
        // The retained project execution and every decoded text place share one
        // live symbol generation across all producer and consumer batches.
        let mut symbols = Symbols::default();
        for batch in run.batches {
            // Each authored occurrence in batch order; neither repeated batch entries nor repeated occurrences are deduplicated by rule.
            let declared = batch
                .iter()
                .flat_map(|rule| run.todo.iter().filter(move |todo| todo.rule == *rule))
                .collect::<Vec<&Todo>>();
            if declared.is_empty() {
                continue;
            }
            let bindings = declared
                .iter()
                .map(|todo| bind(&mut stage, todo))
                .collect::<Vec<_>>();
            let (results, refused) = {
                // The occurrences whose bindings hold reach the phase; a refused binding stays with its occurrence below.
                let submitted = declared
                    .iter()
                    .zip(&bindings)
                    .filter_map(|(todo, binding)| match binding {
                        Ok(binding) => Some((*todo, binding)),
                        Err(_) => None,
                    })
                    .collect::<Vec<_>>();
                let occurrences = submitted
                    .iter()
                    .map(|(todo, binding)| PhaseOccurrence {
                        rule: todo.rule,
                        bound: &binding.bound,
                        slot: binding.slot,
                        demand: FrontierDemand::Observations {
                            ready: &binding.ready,
                        },
                    })
                    .collect::<Vec<_>>();
                let crossing = ProjectCrossing {
                    arenas: stage.arenas(),
                    program: Arc::clone(&run.program),
                    domain: run.domain.clone(),
                };
                let submitted_todo = |occurrence: OccurrenceNumber| {
                    submitted
                        .get(occurrence.0)
                        .map(|(todo, _)| *todo)
                        .ok_or(OperatingError::NotRun)
                };
                let ran = stage.evaluate_occurrences(
                    PhaseOccurrences {
                        occurrences: &occurrences,
                        first: run.first_bucket,
                        corpus: run.domain.files,
                        covered: run.domain.covered,
                        skipped: run.domain.skipped,
                        inapplicable: &inapplicable,
                        facts: run.facts,
                        project: run.project,
                        project_symbols: run.project_symbols,
                        arenas: crossing.arenas,
                    },
                    &mut symbols,
                    |_| Ok::<(), CrossFileError>(()),
                    |occurrence| phase_partials(&crossing, submitted_todo(occurrence)?),
                    &mut guard,
                    |occurrence, outcome, slot, symbols: &mut Symbols| {
                        self.complete_occurrence(&crossing, submitted_todo(occurrence)?, outcome, slot, symbols)
                    },
                );
                match ran {
                    ProjectBatch::Complete(completed) => (completed, None),
                    ProjectBatch::Refused { completed, error } => (completed, Some(error)),
                }
            };
            let mut first_error = None;
            let mut outcomes = results.into_iter();
            // Past the guard's refusal no occurrence has an outcome to deliver; every one is still retired.
            let mut delivering = true;
            for (todo, binding) in declared.iter().zip(bindings) {
                let outcome = match binding {
                    Err(error) if delivering => Some(Err(error)),
                    Err(_) => None,
                    Ok(_) => {
                        let outcome = outcomes.next();
                        delivering = outcome.is_some();
                        outcome.map(|result| result.map_err(|stop| stopped(todo, stop)))
                    }
                };
                if let Some(outcome) = outcome {
                    let delivered = outcome.and_then(|result| deliver(&mut stage, todo, result, answered));
                    if let Err(error) = delivered {
                        first_error.get_or_insert(error);
                    }
                }
                if let Err(error) = stage.finish(todo.rule) {
                    first_error.get_or_insert(CrossFileError::from(error));
                }
            }
            if let Some(error) = refused {
                first_error.get_or_insert(error);
            }
            if let Some(error) = first_error {
                let _finished = stage.finish_stage();
                return Err(error);
            }
        }
        stage.finish_stage()?;
        Ok(())
    }
}

/// Settle one delivered occurrence into the answers and install its publications on the stage.
fn deliver(
    stage: &mut impl ObservationPublication,
    todo: &Todo,
    result: CrossedWithPublications,
    answered: &mut Answered,
) -> Result<(), CrossFileError> {
    let Some(transfer) = settle(
        todo,
        result.crossed,
        &mut answered.later,
        &mut answered.fed,
        &mut answered.unknowns,
    ) else {
        return Ok(());
    };
    let publications = result
        .publications
        .into_iter()
        .map(|each| AdmittedOutput {
            output: each.output,
            publication: each.publication,
        })
        .collect();
    stage.install(todo.rule, result.slot, transfer, publications)?;
    Ok(())
}

/// The partials one occurrence of a phase is evaluated with: those its diagnostic sees, when findings are answered, and those its
/// publication sees.
pub(super) fn phase_partials(crossing: &ProjectCrossing<'_>, todo: &Todo) -> Result<PhasePartials, CrossFileError> {
    let evaluation = match crossing.answering {
        Answering::Findings => partials_for(&MetadataUseCaseImpl, &crossing.domain, todo.rule),
        Answering::Rows(_) => Default::default(),
    };
    let reads = crossing
        .program
        .observation_reads(todo.rule)
        .map_err(|error| CrossFileError::from(failed(&todo.id, &error)))?;
    let publication = partials_for_reads(&MetadataUseCaseImpl, &crossing.domain, todo.rule, &reads);
    Ok(PhasePartials {
        evaluation,
        publication,
    })
}

/// What binds one declared occurrence of an observation batch: the readiness of its frontiers, the publication slot it reserves and
/// its bound inputs, in that order. The stage proves which producers of the rule's frontiers are published; the engine reads that
/// proof, not a choice of ours.
fn bind(stage: &mut impl ObservationExecution, todo: &Todo) -> Result<BatchOccurrenceBinding, CrossFileError> {
    let ready = stage.frontier_readiness(todo.rule)?;
    let slot = stage.reserve(todo.rule)?;
    let bound = stage.bindings(todo.rule)?;
    Ok(BatchOccurrenceBinding { ready, slot, bound })
}

/// Why one occurrence has no completed outcome, named for its rule as the phase's steps name it.
pub(super) fn stopped(todo: &Todo, stop: PhaseStop<CrossFileError>) -> CrossFileError {
    match stop {
        PhaseStop::Caller(error) => error,
        PhaseStop::Operating(error) => CrossFileError::from(failed(&todo.id, &error)),
        PhaseStop::Frontiers(PreparationError::Facts(error)) => CrossFileError::from(error),
        PhaseStop::Frontiers(PreparationError::Metadata(error)) => {
            CrossFileError::from(failed(&todo.id, &error))
        }
        PhaseStop::MissingFirstBucket => CrossFileError::from(failed(&todo.id, &MISSING_FRONTIER_BUCKET)),
        PhaseStop::Unevaluated => CrossFileError::from(failed(&todo.id, &MISSING_OUTCOME)),
    }
}
