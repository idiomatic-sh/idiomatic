//! The ordinary project phase: the phase opened, one call that prepares each declared rule within the budget, evaluates the batch
//! and completes each outcome, then the phase released.
use super::super::constants::CROSS_RULE_MEMORY;
use ryo_values::domain::table_words::constants::LIFECYCLE_TARGET;
use super::super::errors::CrossFileError;
use super::super::service::{CrossFile, phase_partials, stopped};
use super::super::types::{OrdinaryOutcome, OrdinaryRun, ProjectOutcome};
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use crate::domain::budget::service::within;
use crate::domain::crossing::service::inapplicable_files;
use crate::outbound::memory::interface::MemoryPort;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest as _;
use ryo_execution::inbound::use_cases::execution::interface::FrontierDemand;
use ryo_execution::inbound::use_cases::execution::interface::OccurrenceNumber;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::PhaseOccurrence;
use ryo_execution::inbound::use_cases::execution::interface::PhaseOccurrences;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution as _;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage as _;
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
        let occurrences = run
            .todo
            .iter()
            .map(|todo| PhaseOccurrence {
                rule: todo.rule,
                bound: &[],
                slot: PublicationSlot::Unused,
                demand: FrontierDemand::Diagnostic,
            })
            .collect::<Vec<_>>();
        let declared =
            |occurrence: OccurrenceNumber| run.todo.get(occurrence.0).ok_or(OperatingError::NotRun);
        let stage = run
            .services
            .project_phase()
            .map_err(|error| failed(&first_rule.id, &error))?;
        let ran = stage.evaluate_occurrences(
            PhaseOccurrences {
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
            |occurrence| phase_partials(crossing, declared(occurrence)?),
            |rule: RuleIndex| -> Result<(), CrossFileError> {
                let id = run.ids.get(rule.0).ok_or(OperatingError::NotRun)?;
                Ok(within(memory.resident_bytes()?, run.budget, || vec![id.clone()])?)
            },
            |occurrence, outcome, slot, symbols: &mut Symbols| {
                self.complete_occurrence(crossing, declared(occurrence)?, outcome, slot, symbols)
            },
        );
        let (occurrences, refused) = match ran {
            ProjectBatch::Complete(completed) => (completed, None),
            ProjectBatch::Refused { completed, error } => (completed, Some(error)),
        };
        let outcomes = occurrences
            .into_iter()
            .zip(run.todo)
            .map(|(result, todo)| ProjectOutcome {
                result: result.map_err(|stop| stopped(todo, stop)),
                todo: todo.clone(),
            })
            .collect();
        // Every scope and outcome of the phase is released above, so the phase owns its publication alone again.
        let finished = stage.finish();
        // The caller keeps an earlier rule or budget failure over a refusal of the phase's release.
        let finished = finished.map_err(|error| CrossFileError::from(failed(&first_rule.id, &error)));
        Ok(Some(OrdinaryOutcome {
            outcomes,
            refused,
            finished,
            symbols: cross,
        }))
    }
}
