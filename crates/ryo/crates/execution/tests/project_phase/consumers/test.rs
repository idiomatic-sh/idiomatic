//! Same producer proof, distinct resolved input slots, one project batch protocol.
use super::super::*;
use ryo_execution::inbound::use_cases::execution::interface::{
    ObservationExecution, ObservationPublication, ProjectRootExecution,
};
use ryo_values::domain::{
    observations::types::*,
    records::types::{CrossPublicationTransfer, PackedObservedRoot},
    stored_facts::types::{ArenaBytes, Digest, KeptWords},
    table_words::{
        service::{encode_table, pack},
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
        authored.plans[0].steps = vec![PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::Relation(relation),
                comparisons: vec![],
                kept: vec![ColumnIndex(u32::try_from(file_column).unwrap())],
            },
        }];
        authored.plans[0].frontier = vec![Input::Relation(relation)];
        authored.plans[0].observations = vec![PlanObservation {
            name: ObservationName("files".into()),
            source: Input::Step(StepId(0)),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        }];
        let producer = authored.rules[0].id.clone();
        for consumer in [1, 2] {
            authored.plans[consumer].steps = vec![PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Select {
                    from: Input::BoundObservation(ObservationBindingId(0)),
                    comparisons: vec![],
                    kept: vec![],
                },
            }];
            authored.plans[consumer].frontier =
                vec![Input::BoundObservation(ObservationBindingId(0))];
            authored.plans[consumer].observation_inputs = vec![BoundObservationInput {
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
        let request = engine.begin(&prepared, &corpus);
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
        let producer_scope = stage
            .project_scope(RuleIndex(0), &corpus, &covered, &[])
            .unwrap();
        let slot = stage.reserve(RuleIndex(0)).unwrap();
        let ordinary = request
            .prepare_frontiers(
                FrontierRequest {
                    scope: &producer_scope,
                    rule: RuleIndex(0),
                    first: Bucket(0),
                    facts: &stored,
                    project: &project,
                    project_symbols: &project_symbols,
                    demand: FrontierDemand::Diagnostic,
                },
                &mut symbols,
            )
            .unwrap();
        let ProjectBatch::Complete(mut produced) = stage.evaluate(
            vec![ProjectTask {
                rule: RuleIndex(0),
                demand: RuleDemand {
                    emit: EmitDemand::Disabled,
                    observations: ObservationDemand::Required,
                },
                input: ProjectInput {
                    scope: &producer_scope,
                    evaluation: &partials,
                    publication: &partials,
                    ordinary,
                    bound: &[],
                    arenas: stage.arenas(),
                },
            }],
            &mut symbols,
            |_| Ok::<(), std::convert::Infallible>(()),
        ) else {
            panic!("producer guard cannot refuse")
        };
        let mut produced = produced.pop().unwrap().unwrap();
        assert!(matches!(produced.emit, RootOutcome::NoQuery));
        let (output, root) = produced.observations.pop().unwrap();
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
        ryo_execution::inbound::use_cases::publication::interface::PublicationUseCase::publication(
            &ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new(
            ),
            ryo_execution::inbound::use_cases::publication::types::PublicationInput {
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
                        words: Some(KeptWords(pack(WordSlice(&words.0)))),
                        readiness: ProducerReadiness::Complete,
                        arenas: vec![],
                    }],
                },
                vec![(output, publication)],
            )
            .unwrap();
        stage.finish(RuleIndex(0)).unwrap();
        let reads = project.reads.load(Ordering::SeqCst);
        let mut scopes = vec![];
        let mut frontiers = vec![];
        let mut bindings = vec![];
        for rule in order {
            let scope = stage
                .project_scope(RuleIndex(rule), &corpus, &covered, &[])
                .unwrap();
            let ordinary = request
                .prepare_frontiers(
                    FrontierRequest {
                        scope: &scope,
                        rule: RuleIndex(rule),
                        first: Bucket(u32::try_from(rule).unwrap()),
                        facts: &stored,
                        project: &project,
                        project_symbols: &project_symbols,
                        demand: FrontierDemand::Diagnostic,
                    },
                    &mut symbols,
                )
                .unwrap();
            bindings.push(stage.bindings(RuleIndex(rule)).unwrap());
            scopes.push(scope);
            frontiers.push(ordinary);
        }
        let tasks = order
            .into_iter()
            .zip(&scopes)
            .zip(frontiers)
            .zip(&bindings)
            .map(|(((rule, scope), ordinary), bound)| ProjectTask {
                rule: RuleIndex(rule),
                demand: RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::NotRequired,
                },
                input: ProjectInput {
                    scope,
                    evaluation: &partials,
                    publication: &partials,
                    ordinary,
                    bound,
                    arenas: stage.arenas(),
                },
            })
            .collect();
        let ProjectBatch::Complete(results) = stage.evaluate(tasks, &mut symbols, |_| {
            Ok::<(), std::convert::Infallible>(())
        }) else {
            panic!("consumer guard cannot refuse")
        };
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
        drop(bindings);
        drop(scopes);
        drop(producer_scope);
        stage.finish(RuleIndex(1)).unwrap();
        stage.finish(RuleIndex(2)).unwrap();
        stage
            .finish_stage()
            .expect("all actual scope/read obligations settle");
    }
}
