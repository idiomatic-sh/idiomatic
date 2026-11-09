//! Cross-file evaluation of the project batches over injected ports.
use super::completion::service::ProjectCompletion;
use super::completion::service::collected;
use super::completion::types::CrossedWithPublications;
use super::constants::{MISSING_FRONTIER_BUCKET, MISSING_OUTCOME};
use super::errors::CrossFileError;
use super::rendering::service::Rendering;
use super::types::ObservationRun;
use super::types::ProjectCrossing;
use crate::domain::answering::service::settle;
use crate::domain::answering::types::Answered;
use crate::domain::answering::types::Todo;
use crate::domain::crossing::service::inapplicable_files;
use crate::domain::crossing::service::partials_for;
use crate::domain::crossing::service::partials_for_reads;
use crate::domain::crossing::service::refused_files;
use crate::domain::render::types::Answering;
use crate::outbound::corpus::interface::CorpusPort;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::FileObservationReadiness;
use ryo_execution::inbound::use_cases::execution::interface::ObservationExecution;
use ryo_execution::inbound::use_cases::execution::interface::OccurrenceNumber;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseStop;
use ryo_execution::inbound::use_cases::execution::interface::PreparationError;
use ryo_execution::inbound::use_cases::execution::interface::ProjectOutputOutcome;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::RuleOutcome;
use ryo_execution::inbound::use_cases::execution::interface::AdmittedOutput;
use ryo_execution::inbound::use_cases::execution::interface::BatchNumber;
use ryo_execution::inbound::use_cases::execution::interface::Install;
use ryo_execution::inbound::use_cases::execution::interface::ObservationBatches;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution as _;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram;
use crate::domain::crossing::types::CrossingInput;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_execution::inbound::use_cases::publication::interface::PublicationUseCase;
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
    /// the arena of the worker its publication slot reserved.
    pub(super) fn complete_occurrence(
        &self,
        crossing: &ProjectCrossing<'_>,
        todo: &Todo,
        outcome: RuleOutcome<ProjectOutputOutcome>,
        worker: Option<WorkerNumber>,
        symbols: &mut Symbols,
    ) -> Result<CrossedWithPublications, CrossFileError> {
        let evaluated =
            collected(outcome).map_err(|error| CrossFileError::from(failed(&todo.id, &error)))?;
        self.completion
            .complete_phase(crossing, todo, evaluated, symbols, worker)
            .map_err(CrossFileError::from)
    }

    /// Execute the declared ready batches and transfer publications before their final consumer retires.
    pub(crate) fn observations(
        &self,
        run: ObservationRun<'_>,
        arenas: &mut Vec<ArenaBytes>,
        answered: &mut Answered,
        guard: impl FnMut(RuleIndex) -> Result<(), CrossFileError>,
    ) -> Result<(), CrossFileError> {
        let stage = run.services.observations(arenas)?;
        let rules = (0..run.domain.metadata.len())
            .map(RuleIndex)
            .collect::<Vec<_>>();
        // Each authored occurrence in batch order, numbered by its place here; neither repeated batch entries nor repeated
        // occurrences are deduplicated by rule.
        let declared = run
            .batches
            .iter()
            .enumerate()
            .flat_map(|(batch, rules)| {
                rules.iter().flat_map(move |rule| {
                    run.todo
                        .iter()
                        .filter(move |todo| todo.rule == *rule)
                        .map(move |todo| (BatchNumber(batch), todo))
                })
            })
            .collect::<Vec<(BatchNumber, &Todo)>>();
        let occurrences = declared
            .iter()
            .map(|(batch, todo)| (*batch, todo.rule))
            .collect::<Vec<_>>();
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
        let refused = refused_files(&run.domain);
        let todo = |occurrence: OccurrenceNumber| {
            declared
                .get(occurrence.0)
                .map(|(_, todo)| *todo)
                .ok_or(OperatingError::NotRun)
        };
        // The retained project execution and every decoded text place share one
        // live symbol generation across all producer and consumer batches.
        let mut symbols = Symbols::default();
        stage.run_batches(
            ObservationBatches {
                occurrences: &occurrences,
                readiness: &readiness,
                rules: &rules,
                first: run.first_bucket,
                corpus: run.domain.files,
                covered: run.domain.covered,
                skipped: run.domain.skipped,
                inapplicable: &inapplicable,
                refused: &refused,
                facts: run.facts,
                project: run.project,
                project_symbols: run.project_symbols,
            },
            &mut symbols,
            |_| Ok::<(), CrossFileError>(()),
            |occurrence| phase_partials(&run.program, &run.domain, todo(occurrence)?),
            guard,
            |occurrence, outcome, arenas, worker, symbols: &mut Symbols| {
                let crossing = ProjectCrossing {
                    arenas,
                    program: Arc::clone(&run.program),
                    domain: run.domain.clone(),
                };
                self.complete_occurrence(&crossing, todo(occurrence)?, outcome, worker, symbols)
            },
            |occurrence, result| {
                let todo = todo(occurrence)?;
                deliver(result.map_err(|stop| stopped(todo, stop))?, answered)
            },
        )
    }
}

/// Settle one delivered occurrence into the answers; what it publishes, the stage installs.
fn deliver(
    result: CrossedWithPublications,
    answered: &mut Answered,
) -> Result<Option<Install>, CrossFileError> {
    let Some(transfer) = settle(
        result.crossed,
        &mut answered.later,
        &mut answered.unknowns,
    ) else {
        return Ok(None);
    };
    let publications = result
        .publications
        .into_iter()
        .map(|each| AdmittedOutput {
            output: each.output,
            publication: each.publication,
        })
        .collect();
    Ok(Some(Install {
        transfer,
        publications,
    }))
}

/// The partials one occurrence of a phase is evaluated with: those its diagnostic sees, when findings are answered, and those its
/// publication sees.
pub(super) fn phase_partials(
    program: &PreparedExecutionProgram,
    domain: &CrossingInput<'_>,
    todo: &Todo,
) -> Result<PhasePartials, CrossFileError> {
    let evaluation = match domain.answering {
        Answering::Findings => partials_for(&MetadataUseCaseImpl, domain, todo.rule),
        Answering::Rows(_) => Default::default(),
    };
    let reads = program
        .observation_reads(todo.rule)
        .map_err(|error| CrossFileError::from(failed(&todo.id, &error)))?;
    let publication = partials_for_reads(&MetadataUseCaseImpl, domain, todo.rule, &reads);
    Ok(PhasePartials {
        evaluation,
        publication,
    })
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
        PhaseStop::MissingFirstBucket => {
            CrossFileError::from(failed(&todo.id, &MISSING_FRONTIER_BUCKET))
        }
        PhaseStop::Unevaluated => CrossFileError::from(failed(&todo.id, &MISSING_OUTCOME)),
        PhaseStop::Unbound(error) => CrossFileError::from(error),
    }
}
