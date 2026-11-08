use super::program::{corpus, engine, prepared};
use super::subscriber::{Access, Activation, Sample};
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::application::file_dag::frontiers::types::PublicationSlot;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::{
    FrontierDemand, PhaseOccurrence, PhaseOccurrences, PhasePartials,
};
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::interface::FactsFinish;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use ryo_execution_runtime::outbound::facts::types::ColumnsMut;
use ryo_execution_runtime::outbound::facts::types::KeptBucket;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution_runtime::outbound::facts::types::WordsToKeep;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{ColumnsView, FileId, Relation};
use ryo_values::domain::stored_facts::types::Bucket;

/// Compiled frontiers are empty, so no source row or kept bucket may be requested.
struct NoFacts;
impl FactsFinish for NoFacts {
    fn kept(&self, _: Bucket) -> Result<KeptBucket, FactsError> {
        panic!("an activation rule has no file frontier to fetch")
    }
}
impl FactsWorker for NoFacts {
    fn resident_stability(&self) -> ResidentStability {
        ResidentStability::Stable
    }
    fn rows(&self, _: Relation) -> Result<ColumnsView<'_>, FactsError> {
        panic!("an activation rule has no project relation to read")
    }
    fn columns_mut(&mut self) -> Result<ColumnsMut<'_>, FactsError> {
        panic!("an activation rule writes no facts")
    }
    fn keep(&self, _: Bucket, _: WordsToKeep<'_>) -> Result<(), FactsError> {
        panic!("an activation rule writes no facts")
    }
    fn release(&mut self) -> Result<(), FactsError> {
        panic!("an activation rule writes no facts")
    }
    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError> {
        panic!("an activation rule writes no facts")
    }
}

/// The public project entry for the declared roots in `demanded` (ascending), observed from
/// phase opening to the last delivery. `intrusion` runs inside the observed window and stands
/// for work no production counter reports; the allocator still sees what it allocates.
pub(crate) fn measure_project(
    total: usize,
    demanded: &[usize],
    intrusion: &dyn Fn(usize),
) -> (Sample, Access) {
    let engine = engine();
    let prepared = prepared(&engine, total);
    let corpus = corpus();
    let covered = [Covered {
        file: FileId(0),
        rules: demanded.iter().copied().map(RuleIndex).collect(),
        partial: vec![],
    }];
    let observed = Activation::default();
    let dispatch = tracing::Dispatch::new(observed.clone());
    let project_symbols = Symbols::default();
    // The first cycle warms callsite registration and allocator metadata; the second
    // sees exactly the same real request and declared roots.
    // The per-rule inputs of the phase are built before the measured cycle: they are the caller's, not activation's.
    let first = vec![Bucket(0); total];
    let inapplicable =
        vec![ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(); total];
    let refused =
        vec![ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default(); total];
    for _ in 0..2 {
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
        let partials = Partials::default();
        let mut symbols = Symbols::default();
        tracing::dispatcher::with_default(&dispatch, || {
            tracing::trace!(target: "ryo::diagnostic::activation", event = "begin");
            let phase = request.project_phase().unwrap();
            let occurrences: Vec<PhaseOccurrence<'_>> = demanded
                .iter()
                .map(|&rule| PhaseOccurrence {
                    rule: RuleIndex(rule),
                    bound: &[],
                    slot: PublicationSlot::Unused,
                    demand: FrontierDemand::Diagnostic,
                })
                .collect();
            let ProjectBatch::Complete(outcomes) = phase.evaluate_occurrences(
                PhaseOccurrences {
                    occurrences: &occurrences,
                    first: &first,
                    corpus: &corpus,
                    covered: &covered,
                    skipped: &[],
                    inapplicable: &inapplicable,
                    refused: &refused,
                    facts: &NoFacts,
                    project: &NoFacts,
                    project_symbols: &project_symbols,
                    arenas: &[],
                },
                &mut symbols,
                |_| Ok::<(), std::convert::Infallible>(()),
                |_| {
                    Ok(PhasePartials {
                        evaluation: partials.clone(),
                        publication: partials.clone(),
                    })
                },
                |_| Ok(()),
                |_, outcome, _, _| Ok(outcome),
            ) else {
                panic!("an infallible guard cannot refuse")
            };
            assert_eq!(outcomes.len(), demanded.len());
            for outcome in outcomes {
                outcome.unwrap();
            }
            intrusion(total);
            tracing::trace!(target: "ryo::diagnostic::activation", event = "end");
            phase.finish().unwrap();
        });
        request.finish();
    }
    let state = observed.0.lock().unwrap();
    assert_eq!((state.starts, state.ends, state.active), (2, 2, false));
    (state.sample, state.access)
}
