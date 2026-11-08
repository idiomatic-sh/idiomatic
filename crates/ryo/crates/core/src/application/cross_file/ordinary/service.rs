//! The ordinary project phase: one engine call that prepares each declared rule within the budget, evaluates the batch, completes
//! each outcome and releases the phase.
use super::super::constants::{CROSS_RULE_MEMORY, MISSING_FRONTIER_BUCKET, MISSING_OUTCOME};
use ryo_values::domain::table_words::constants::LIFECYCLE_TARGET;
use super::super::completion::service::collected;
use super::super::errors::CrossFileError;
use super::super::service::CrossFile;
use super::super::types::{OrdinaryOutcome, OrdinaryRun, ProjectOutcome};
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use crate::domain::budget::service::within;
use crate::domain::crossing::service::{inapplicable_files, partials_for, partials_for_reads};
use crate::domain::render::types::Answering;
use crate::outbound::memory::interface::MemoryPort;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest as _;
use ryo_execution::inbound::use_cases::execution::interface::OccurrenceNumber;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::OrdinaryPhase;
use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseStop;
use ryo_execution::inbound::use_cases::execution::interface::PreparationError;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution as _;
use ryo_execution::inbound::use_cases::execution::interface::PublicationSlot;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::rule_requests::service::failed;

impl CrossFile {
    /// Makes the ordinary rules that cross files in one phase: they are independent of one another and share one interpreter and
    /// symbol generation. Answers each rule's outcome in the order of `run.todo`, the refusal of the budget guard, and the release
    /// of the stage, for the caller to settle.
    pub(crate) fn ordinary(
        &self,
        run: OrdinaryRun<'_>,
        memory: &dyn MemoryPort,
    ) -> Result<Option<OrdinaryOutcome>, CrossFileError> {
        let Some(first_rule) = run.todo.first() else {
            return Ok(None);
        };
        within(memory.resident_bytes()?, run.budget, || vec![first_rule.id.clone()])?;
        // All ordinary project rules use the same interpreter and symbol generation.
        let mut cross = Symbols::default();
        let crossing = &run.crossing;
        let inapplicable = inapplicable_files(&MetadataUseCaseImpl, &crossing.domain);
        let occurrences = run.todo.iter().map(|todo| todo.rule).collect::<Vec<_>>();
        let declared =
            |occurrence: OccurrenceNumber| run.todo.get(occurrence.0).ok_or(OperatingError::NotRun);
        let ran = run
            .services
            .ordinary_phase(
                OrdinaryPhase {
                    occurrences: &occurrences,
                    first: run.first_bucket,
                    corpus: crossing.domain.files,
                    covered: crossing.domain.covered,
                    skipped: crossing.domain.skipped,
                    inapplicable: &inapplicable,
                    facts: run.facts,
                    project: run.project,
                    project_symbols: run.project_symbols,
                    arenas: crossing.arenas,
                },
                &mut cross,
                // The first rule was checked before the phase opened; every later one is checked before its preparation.
                |occurrence| -> Result<_, CrossFileError> {
                    let each = declared(occurrence)?;
                    if occurrence.0 != 0 {
                        within(memory.resident_bytes()?, run.budget, || vec![each.id.clone()])?;
                    }
                    // Rule identity on the decoder events of the opt-in lifecycle memory diagnostics.
                    Ok(tracing::enabled!(target: LIFECYCLE_TARGET, tracing::Level::TRACE).then(|| {
                        tracing::trace_span!(target: LIFECYCLE_TARGET, CROSS_RULE_MEMORY, rule = %each.id.0, rule_index = each.rule.0)
                            .entered()
                    }))
                },
                |occurrence| -> Result<PhasePartials, CrossFileError> {
                    let todo = declared(occurrence)?;
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
                },
                |rule: RuleIndex| -> Result<(), CrossFileError> {
                    let id = run.ids.get(rule.0).ok_or(OperatingError::NotRun)?;
                    Ok(within(memory.resident_bytes()?, run.budget, || vec![id.clone()])?)
                },
                |occurrence, outcome, symbols: &mut Symbols| -> Result<_, CrossFileError> {
                    let todo = declared(occurrence)?;
                    let evaluated = collected(outcome)
                        .map_err(|error| CrossFileError::from(failed(&todo.id, &error)))?;
                    self.completion
                        .complete_phase(crossing, todo, evaluated, symbols, PublicationSlot::Unused)
                        .map_err(CrossFileError::from)
                },
            )
            .map_err(|error| failed(&first_rule.id, &error))?;
        let outcomes = ran
            .occurrences
            .into_iter()
            .zip(run.todo)
            .map(|(result, todo)| ProjectOutcome {
                result: result.map_err(|stop| stopped(todo, stop)),
                todo: todo.clone(),
            })
            .collect();
        // The caller keeps an earlier rule or budget failure over a refusal of the phase's release.
        let finished = ran
            .finished
            .map_err(|error| CrossFileError::from(failed(&first_rule.id, &error)));
        Ok(Some(OrdinaryOutcome {
            outcomes,
            refused: ran.refused,
            finished,
            symbols: cross,
        }))
    }
}

/// Why one occurrence has no completed outcome, named for its rule as the phase's steps name it.
fn stopped(todo: &Todo, stop: PhaseStop<CrossFileError>) -> CrossFileError {
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
