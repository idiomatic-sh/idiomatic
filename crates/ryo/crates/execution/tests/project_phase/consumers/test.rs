//! Same producer proof, distinct resolved input slots, one project batch protocol.
use super::super::*;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ObservationExecution;
use ryo_execution::inbound::use_cases::execution::interface::ObservationPublication;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::observation_stage::types::AdmittedOutput;
use ryo_values::domain::{
    observations::types::*,
    plan_parts::types::ObservationBindingId,
    records::types::{
        CrossPublicationTransfer, ObservationReadinessLayout, PackedObservedRoot, ProducerReadiness,
    },
    stored_facts::types::{ArenaBytes, Digest, KeptWords},
    table_words::{
        fixtures::{encode_table, pack},
        types::WordSlice,
    },
};

#[test]
fn two_project_consumers_bind_their_own_slots_to_the_same_producer_in_both_orders() {
    for order in [[1, 2], [2, 1]] {
        let engine = engine();
        let mut authored = authored_request(3);
        let relation = Relation::ModuleIdentity;
        let file_column = kinds(relation)
            .iter()
            .position(|kind| *kind == CellKind::File)
            .unwrap();
        let columns = vec![ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        }];
        crate::boundary_fixture::plans(&mut authored)[0].steps = vec![PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::Relation(relation),
                comparisons: vec![],
                kept: vec![ColumnIndex(u32::try_from(file_column).unwrap())],
            },
        }];
        crate::boundary_fixture::plans(&mut authored)[0].frontier = vec![Input::Relation(relation)];
        crate::boundary_fixture::plans(&mut authored)[0].observations = vec![PlanObservation {
            name: ObservationName("files".into()),
            source: Input::Step(StepId(0)),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        }];
        let producer = authored.rules[0].id.clone();
        for consumer in [1, 2] {
            crate::boundary_fixture::plans(&mut authored)[consumer].steps = vec![PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Select {
                    from: Input::BoundObservation(ObservationBindingId(0)),
                    comparisons: vec![],
                    kept: vec![],
                },
            }];
            crate::boundary_fixture::plans(&mut authored)[consumer].frontier =
                vec![Input::BoundObservation(ObservationBindingId(0))];
            crate::boundary_fixture::plans(&mut authored)[consumer].observation_inputs =
                vec![BoundObservationInput {
                    name: ObservationBindingName("files".into()),
                    columns: columns.clone(),
                    scope: StepScope::CrossFile,
                }];
            authored.rules[consumer].observations = vec![ObservationBinding {
                name: ObservationBindingName("files".into()),
                columns: columns.clone(),
                producers: vec![ObservationProducer::Output {
                    rule: producer.clone(),
                    output: ObservationName("files".into()),
                }],
            }];
        }
        authored.observations =
            ObservationIdentity::Named(ObservationReadinessLayout(Digest([71; 16])));
        let prepared = engine.prepare(authored).unwrap();
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
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
        let mut stage = request.observations(&mut arenas).unwrap();
        let mut symbols = Symbols::default();
        let partials = Partials::default();
        let slot = stage.reserve(RuleIndex(0)).unwrap();
        let first = [Bucket(0), Bucket(1), Bucket(2)];
        let inapplicable = vec![
                ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(
                );
                3
            ];
        let mut produced = crate::boundary_fixture::project_root(
            &stage,
            PhaseOccurrences {
                occurrences: &[PhaseOccurrence {
                    rule: RuleIndex(0),
                    bound: &[],
                    slot,
                    demand: FrontierDemand::Diagnostic,
                }],
                first: &first,
                corpus: &corpus,
                covered: &covered,
                skipped: &[],
                inapplicable: &inapplicable,
                refused: &vec![ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default(); inapplicable.len()],
                facts: &stored,
                project: &project,
                project_symbols: &project_symbols,
                arenas: stage.arenas(),
            },
            &partials,
            &partials,
            &mut symbols,
        )
        .unwrap();
        // The phase call always demands the emit; that an undemanded emit answers `NoQuery` is proved in the runtime by
        // `project_phase::service_test::a_root_whose_emit_is_not_demanded_answers_no_query`.
        assert!(
            !matches!(
                produced.emit,
                RootOutcome::Failed(_) | RootOutcome::Unknown(_)
            ),
            "the phase call evaluates the producer's own emit beside its publication"
        );
        let ProjectOutputOutcome {
            output,
            outcome: root,
        } = produced.observations.pop().unwrap();
        let ProjectObservation::Complete {
            mut table,
            publication,
        } = root
        else {
            panic!("real complete producer")
        };
        assert_eq!(table.cells, [Value::File(FileId(0))]);
        let PublicationSlot::Reserved(worker) = slot else {
            panic!("reserved producer slot")
        };
        let mut arena = ArenaBytes::default();
        let mut places = vec![];
        let mut words = Words::default();
        ryo_execution::application::publication::service::publication(
            ryo_execution::application::publication::types::PublicationInput {
                table: &mut table,
                symbols: &mut symbols,
                inherited: stage.arenas(),
                worker,
                arena: &mut arena,
                places: &mut places,
            },
        )
        .unwrap();
        encode_table(&table, &places, &mut words);
        stage
            .install(
                RuleIndex(0),
                slot,
                CrossPublicationTransfer {
                    worker,
                    arena,
                    roots: vec![PackedObservedRoot {
                        output,
                        words: ryo_values::domain::records::types::RootWords::Published(KeptWords(
                            pack(WordSlice(&words.0)),
                        )),
                        readiness: ProducerReadiness::Complete,
                        arenas: vec![],
                    }],
                },
                vec![AdmittedOutput {
                    output,
                    publication,
                }],
            )
            .unwrap();
        stage.finish(RuleIndex(0)).unwrap();
        let reads = project.reads.load(Ordering::SeqCst);
        let bindings: Vec<_> = order
            .iter()
            .map(|rule| stage.bindings(RuleIndex(*rule)).unwrap())
            .collect();
        let occurrences: Vec<PhaseOccurrence<'_>> = order
            .iter()
            .zip(&bindings)
            .map(|(rule, bound)| PhaseOccurrence {
                rule: RuleIndex(*rule),
                bound,
                slot: PublicationSlot::Unused,
                demand: FrontierDemand::Diagnostic,
            })
            .collect();
        let results = crate::boundary_fixture::project_roots(
            &stage,
            PhaseOccurrences {
                occurrences: &occurrences,
                first: &first,
                corpus: &corpus,
                covered: &covered,
                skipped: &[],
                inapplicable: &inapplicable,
                refused: &vec![ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default(); inapplicable.len()],
                facts: &stored,
                project: &project,
                project_symbols: &project_symbols,
                arenas: stage.arenas(),
            },
            &partials,
            &partials,
            &mut symbols,
        );
        assert_eq!(results.len(), 2);
        for result in results {
            let result = result.unwrap();
            let RootOutcome::Complete(table) = result.emit else {
                panic!("each actual named slot must be supplied")
            };
            assert_eq!(table.kinds, [CellKind::File]);
            assert_eq!(table.count, RowCount(1));
            assert_eq!(table.cells, [Value::File(FileId(0))]);
        }
        assert_eq!(
            project.reads.load(Ordering::SeqCst),
            reads,
            "both consumers reuse the same admitted producer"
        );
        assert_eq!(stored.reads.load(Ordering::SeqCst), 0);
        drop(occurrences);
        drop(bindings);
        stage.finish(RuleIndex(1)).unwrap();
        stage.finish(RuleIndex(2)).unwrap();
        stage
            .finish_stage()
            .expect("all actual scope/read obligations settle");
    }
}
