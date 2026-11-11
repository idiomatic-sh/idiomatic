//! Same producer proof, distinct resolved input slots, one project batch protocol.
use crate::boundary_fixture::fixtures::{corpus, engine};
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::{
    BatchNumber, BatchOccurrence, ObservationBatches,
};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectObservation;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::CapturedFacts;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation, Value};
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::stored_facts::types::Bucket;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn two_project_consumers_bind_their_own_slots_to_the_same_producer_in_both_orders() {
    for order in [[1, 2], [2, 1]] {
        let engine = engine();
        let prepared = super::fixtures::producer_and_consumers(&engine);
        let relation = Relation::ModuleIdentity;
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
        let covered = [Covered {
            file: FileId(0),
            rules: vec![RuleIndex(0), RuleIndex(1), RuleIndex(2)],
            partial: vec![],
        }];
        let stored = CapturedFacts {
            captures: vec![],
            reads: AtomicUsize::new(0),
        };
        let project = ProjectFacts::new(relation);
        let project_symbols = Symbols::default();
        let mut arenas = vec![];
        let stage = request.observations(&mut arenas).unwrap();
        let mut symbols = Symbols::default();
        let partials = Partials::default();
        let first = [Bucket(0), Bucket(1), Bucket(2)];
        let inapplicable = vec![
                ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(
                );
                3
            ];
        let refusals = vec![
            ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default();
            inapplicable.len()
        ];
        // The producer publishes in the first batch; both consumers bind their own slots to it in the second, in `order`.
        let occurrences = [
            BatchOccurrence {
                batch: BatchNumber(0),
                rule: RuleIndex(0),
            },
            BatchOccurrence {
                batch: BatchNumber(1),
                rule: RuleIndex(order[0]),
            },
            BatchOccurrence {
                batch: BatchNumber(1),
                rule: RuleIndex(order[1]),
            },
        ];
        let (ran, delivered) = crate::project_phase::source_facts::fixtures::run_observations(
            stage,
            ObservationBatches {
                occurrences: &occurrences,
                readiness: &[],
                rules: &[RuleIndex(0), RuleIndex(1), RuleIndex(2)],
                first: &first,
                corpus: &corpus,
                covered: &covered,
                skipped: &[],
                inapplicable: &inapplicable,
                refused: &refusals,
                facts: &stored,
                project: &project,
                project_symbols: &project_symbols,
            },
            &partials,
            &mut symbols,
        );
        ran.expect("all actual scope/read obligations settle");
        assert_eq!(
            delivered
                .iter()
                .map(|(number, _)| number.0)
                .collect::<Vec<_>>(),
            [0, 1, 2],
            "every occurrence is delivered in declaration order"
        );
        let mut delivered = delivered.into_iter().map(|(_, outcome)| outcome.unwrap());
        let mut produced = delivered.next().unwrap();
        // The phase call always demands the emit; that an undemanded emit answers `NoQuery` is proved in the runtime by
        // `project_phase::service_test::a_root_whose_emit_is_not_demanded_answers_no_query`.
        assert!(
            !matches!(
                produced.emit,
                RootOutcome::Failed(_) | RootOutcome::Unknown(_)
            ),
            "the phase call evaluates the producer's own emit beside its publication"
        );
        let ProjectOutputOutcome { outcome: root, .. } = produced.observations.pop().unwrap();
        let ProjectObservation::Complete { table, .. } = root else {
            panic!("real complete producer")
        };
        assert_eq!(table.cells, [Value::File(FileId(0))]);
        for result in delivered {
            let RootOutcome::Complete(table) = result.emit else {
                panic!("each actual named slot must be supplied")
            };
            assert_eq!(table.kinds, [CellKind::File]);
            assert_eq!(table.count, RowCount(1));
            assert_eq!(table.cells, [Value::File(FileId(0))]);
        }
        assert_eq!(
            project.reads.load(Ordering::SeqCst),
            1,
            "both consumers reuse the same admitted producer"
        );
        assert_eq!(stored.reads.load(Ordering::SeqCst), 0);
    }
}

/// Every callback of a stage's run is given the occurrence's place among all declared occurrences, not its place in its batch: the
/// second batch's occurrences are numbered 1 and 2 in `before`, `partials`, `complete` and `deliver` alike, and each is delivered in
/// declaration order with the outcome of its own rule (only the producer, numbered 0, observes an output).
#[test]
fn every_callback_numbers_an_occurrence_by_its_place_among_all_declared_occurrences() {
    use ryo_execution::inbound::use_cases::execution::interface::{
        PhasePartials, ProgramObservationStagePort,
    };
    use std::cell::RefCell;
    let engine = engine();
    let prepared = super::fixtures::producer_and_consumers(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0), RuleIndex(1), RuleIndex(2)],
        partial: vec![],
    }];
    let stored = CapturedFacts {
        captures: vec![],
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let project_symbols = Symbols::default();
    let inapplicable = vec![
            ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default();
            3
        ];
    let refusals =
        vec![ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default(); 3];
    let seen = RefCell::new(Vec::new());
    let mut arenas = vec![];
    let stage = request.observations(&mut arenas).unwrap();
    let mut symbols = Symbols::default();
    stage
        .run_batches(
            ObservationBatches {
                occurrences: &[
                    BatchOccurrence {
                        batch: BatchNumber(0),
                        rule: RuleIndex(0),
                    },
                    BatchOccurrence {
                        batch: BatchNumber(1),
                        rule: RuleIndex(1),
                    },
                    BatchOccurrence {
                        batch: BatchNumber(1),
                        rule: RuleIndex(2),
                    },
                ],
                readiness: &[],
                rules: &[RuleIndex(0), RuleIndex(1), RuleIndex(2)],
                first: &[Bucket(0), Bucket(1), Bucket(2)],
                corpus: &corpus,
                covered: &covered,
                skipped: &[],
                inapplicable: &inapplicable,
                refused: &refusals,
                facts: &stored,
                project: &project,
                project_symbols: &project_symbols,
            },
            &mut symbols,
            |number| {
                seen.borrow_mut().push(("before", number.0, 0));
                Ok::<(), OperatingError>(())
            },
            |number| {
                seen.borrow_mut().push(("partials", number.0, 0));
                Ok(PhasePartials {
                    evaluation: Partials::default(),
                    publication: Partials::default(),
                })
            },
            |_| Ok(()),
            |number, outcome, arenas, worker, symbols: &mut Symbols| {
                seen.borrow_mut()
                    .push(("complete", number.0, outcome.observations.len()));
                match worker {
                    Some(worker) => crate::project_phase::source_facts::fixtures::published(
                        &outcome, arenas, worker, symbols,
                    ),
                    None => Ok(None),
                }
            },
            |number, result| {
                seen.borrow_mut().push(("deliver", number.0, 0));
                Ok(result.expect("each occurrence is bound and evaluated"))
            },
        )
        .expect("the stage runs both batches");
    let seen = seen.into_inner();
    let numbers = |callback: &str| {
        seen.iter()
            .filter(|(each, _, _)| *each == callback)
            .map(|(_, number, _)| *number)
            .collect::<Vec<_>>()
    };
    for callback in ["before", "partials", "complete", "deliver"] {
        assert_eq!(numbers(callback), [0, 1, 2], "{callback}");
    }
    assert_eq!(
        seen.iter()
            .filter(|(each, _, _)| *each == "complete")
            .map(|(_, number, observed)| (*number, *observed))
            .collect::<Vec<_>>(),
        [(0, 1), (1, 0), (2, 0)],
        "each number completes its own rule's outcome"
    );
    request.finish();
}

/// A publication is held only as long as the generation it was admitted under: a producer's publication kept from a stage that
/// has finished is refused when a later stage is handed it to install.
#[test]
fn a_publication_held_past_its_stage_is_refused_by_the_next_stage() {
    use std::cell::RefCell;
    let engine = engine();
    let prepared = super::fixtures::producer_and_consumers(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0), RuleIndex(1), RuleIndex(2)],
        partial: vec![],
    }];
    let stored = CapturedFacts {
        captures: vec![],
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let project_symbols = Symbols::default();
    let inapplicable = vec![
            ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default();
            3
        ];
    let refusals =
        vec![ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default(); 3];
    let held = RefCell::new(None);
    let run = |swap: bool| {
        let mut arenas = vec![];
        let stage = request.observations(&mut arenas).unwrap();
        crate::project_phase::source_facts::fixtures::run_observations_with(
            stage,
            ObservationBatches {
                occurrences: &[BatchOccurrence {
                    batch: BatchNumber(0),
                    rule: RuleIndex(0),
                }],
                readiness: &[],
                rules: &[RuleIndex(0), RuleIndex(1), RuleIndex(2)],
                first: &[Bucket(0), Bucket(1), Bucket(2)],
                corpus: &corpus,
                covered: &covered,
                skipped: &[],
                inapplicable: &inapplicable,
                refused: &refusals,
                facts: &stored,
                project: &project,
                project_symbols: &project_symbols,
            },
            &|_| ryo_execution::inbound::use_cases::execution::interface::PhasePartials {
                evaluation: Partials::default(),
                publication: Partials::default(),
            },
            &|_, install| {
                install.map(|mut install| {
                    if swap {
                        for each in &mut install.publications {
                            each.publication =
                                held.borrow().clone().expect("the first stage published");
                        }
                    } else {
                        *held.borrow_mut() = install
                            .publications
                            .first()
                            .map(|each| each.publication.clone());
                    }
                    install
                })
            },
            &mut Symbols::default(),
        )
        .0
    };
    run(false).expect("the first stage publishes and finishes");
    assert!(
        held.borrow().is_some(),
        "the producer handed over its publication"
    );
    assert!(
        matches!(run(true), Err(OperatingError::NotRead)),
        "the held publication's generation ended with its stage"
    );
    request.finish();
}
