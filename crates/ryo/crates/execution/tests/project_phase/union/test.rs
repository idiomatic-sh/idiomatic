//! One project observation union: its members feed the semantic calls, a refused frontier prevents them, and hosted text joins by spelling.
use crate::boundary_fixture::fixtures::{authored_request, corpus};
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileExecution;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::{
    BatchNumber, BatchOccurrence, ObservationBatches,
};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectObservation;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::CapturedFacts;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_execution_runtime::outbound::operators::fixtures::project_phase_union::UnionSemanticCalls;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    fixtures::kinds,
    types::{CellKind, FileId, Relation, Value},
};
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
    ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
};
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::records::types::ObservationReadinessLayout;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::RowCount,
};
use ryo_values::domain::{
    plan_parts::types::{JoinColumn, Side},
    stored_facts::types::Bucket,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn project_observation_union_semantic_calls_use_members_without_a_restored_union() {
    exercise_project_observation_union(false, None);
}

#[test]
fn refused_project_union_frontier_prevents_union_semantic_calls_with_available_members() {
    exercise_project_observation_union(true, None);
}

#[test]
fn project_hosted_text_joins_named_publication_text_by_spelling() {
    for selected in ["F401", "F822"] {
        exercise_project_observation_union(false, Some(selected));
    }
}

fn exercise_project_observation_union(refused: bool, selected_text: Option<&str>) {
    let union_semantic_calls = Arc::new(UnionSemanticCalls::default());
    let engine = PrepareUseCaseImpl::new(
        union_semantic_calls.clone(),
        Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
        std::sync::Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    let mut authored = authored_request(2);
    let relation = Relation::ModuleIdentity;
    let file_column = kinds(relation)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .unwrap();
    let mut columns = vec![ObservationColumn {
        name: ObservationColumnName("file".into()),
        kind: CellKind::File,
    }];
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: Input::Relation(relation),
            comparisons: vec![],
            kept: vec![ColumnIndex(u32::try_from(file_column).unwrap())],
        },
    }];
    if selected_text.is_some() {
        columns.push(ObservationColumn {
            name: ObservationColumnName("code".into()),
            kind: CellKind::Symbol,
        });
        crate::boundary_fixture::fixtures::plans(&mut authored)[0]
            .steps
            .push(PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Extend {
                    from: Input::Step(StepId(0)),
                    values: vec![ryo_values::domain::plan_parts::types::Operand::Literal(
                        ryo_values::domain::plan_parts::types::QueryLiteral::Text(
                            ryo_values::domain::plan_parts::types::LiteralText("F401".into()),
                        ),
                    )],
                },
            });
    }
    crate::boundary_fixture::fixtures::plans(&mut authored)[0]
        .observations
        .push(PlanObservation {
            name: ObservationName("files".into()),
            source: Input::Step(StepId(if selected_text.is_some() { 1 } else { 0 })),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        });
    crate::boundary_fixture::fixtures::plans(&mut authored)[1].steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: Input::BoundObservation(ObservationBindingId(0)),
            comparisons: vec![],
            kept: vec![],
        },
    }];
    crate::boundary_fixture::fixtures::plans(&mut authored)[1]
        .observation_inputs
        .push(BoundObservationInput {
            name: ObservationBindingName("files".into()),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        });
    if let Some(selected) = selected_text {
        use ryo_values::domain::hosted::types::{
            HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
        };
        use ryo_values::domain::plan_parts::types::HostedNumber;
        authored.hosted.push(HostRelation {
            name: HostName("selected-codes".into()),
            columns: vec![HostColumn {
                name: HostColumnName("code".into()),
                kind: HostKind::Text,
            }],
            rows: vec![HostRow(vec![
                ryo_values::domain::row_texts::types::Cell::Text(selected.into()),
            ])],
        });
        crate::boundary_fixture::fixtures::plans(&mut authored)[1]
            .steps
            .extend([
                PlanStep {
                    scope: StepScope::CrossFile,
                    operation: Operation::Hosted {
                        relation: HostedNumber(0),
                    },
                },
                PlanStep {
                    scope: StepScope::CrossFile,
                    operation: Operation::Join {
                        left: Input::Step(StepId(0)),
                        right: Input::Step(StepId(1)),
                        left_on: vec![ColumnIndex(1)],
                        right_on: vec![ColumnIndex(0)],
                        output: vec![JoinColumn {
                            side: Side::Left,
                            column: ColumnIndex(0),
                        }],
                        filter: vec![],
                        view: vec![],
                        reach: None,
                    },
                },
            ]);
        crate::boundary_fixture::fixtures::plans(&mut authored)[1]
            .emit
            .step = StepId(2);
    }
    let producer = authored.rules[0].id.clone();
    authored.rules[1].observations.push(ObservationBinding {
        name: ObservationBindingName("files".into()),
        columns,
        producers: vec![ObservationProducer::Output {
            rule: producer,
            output: ObservationName("files".into()),
        }],
    });
    authored.observations =
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([63; 16])));
    let prepared = engine
        .prepare(authored)
        .expect("actual named project producer and union consumer");
    assert_eq!(
        prepared.frontiers(RuleIndex(1)).unwrap(),
        [Input::BoundObservation(ObservationBindingId(0))]
    );
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0), RuleIndex(1)],
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
    let inapplicable = vec![
            ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default();
            2
        ];
    let refusals = vec![
            ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default();
            inapplicable.len()
        ];
    // The producer publishes in the first batch and the union consumes it in the second; a producer the stage never runs leaves
    // the union's frontier unavailable.
    let occurrences: &[BatchOccurrence] = if refused {
        &[BatchOccurrence {
            batch: BatchNumber(1),
            rule: RuleIndex(1),
        }]
    } else {
        &[
            BatchOccurrence {
                batch: BatchNumber(0),
                rule: RuleIndex(0),
            },
            BatchOccurrence {
                batch: BatchNumber(1),
                rule: RuleIndex(1),
            },
        ]
    };
    let (ran, mut delivered) = crate::project_phase::source_facts::fixtures::run_observations(
        stage,
        ObservationBatches {
            occurrences,
            readiness: &[],
            rules: &[RuleIndex(0), RuleIndex(1)],
            first: &[Bucket(0), Bucket(1)],
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
    ran.unwrap();
    let (_, result) = delivered.pop().unwrap();
    let result = result.unwrap();
    if !refused {
        let (_, produced) = delivered.pop().unwrap();
        let mut produced = produced.unwrap();
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
            outcome: observed,
        } = produced.observations.pop().unwrap();
        assert_eq!(output, ObservationIndex(0));
        let ProjectObservation::Complete { table, .. } = observed else {
            panic!("actual named project publication")
        };
        assert_eq!(
            table.kinds,
            if selected_text.is_some() {
                vec![CellKind::File, CellKind::Symbol]
            } else {
                vec![CellKind::File]
            }
        );
        assert_eq!(table.count, RowCount(1));
        assert_eq!(table.cells[0], Value::File(FileId(0)));
        if selected_text.is_some() {
            assert!(
                matches!(table.cells[1], Value::Name(_)),
                "the project producer actually publishes stable text names"
            );
        } else {
            assert_eq!(table.cells, [Value::File(FileId(0))]);
        }
    }
    assert!(delivered.is_empty(), "one delivery per declared occurrence");
    assert_eq!(
        stored.reads.load(Ordering::SeqCst),
        0,
        "no union bytes were supplied by a file frontier"
    );
    if refused {
        assert!(
            matches!(result.emit, RootOutcome::Unknown(OperatingError::ObservationIncomplete { ref reason })
            if reason == "a producer feeding this retained frontier is unavailable")
        );
    } else {
        let table = match result.emit {
            RootOutcome::Complete(table) => table,
            RootOutcome::Failed(error) | RootOutcome::Unknown(error) => {
                panic!("declared union admission failed: {error:?}");
            }
            RootOutcome::Deferred { phase } => panic!("union deferred to {phase:?}"),
            RootOutcome::Partial { .. } => panic!("union unexpectedly partial"),
            RootOutcome::Skipped(_) => panic!("union unexpectedly skipped"),
            RootOutcome::NoQuery => panic!("union unexpectedly absent"),
        };
        assert_eq!(table.kinds, [CellKind::File]);
        let matches = selected_text.is_none() || selected_text == Some("F401");
        assert_eq!(table.count, RowCount(usize::from(matches)));
        assert_eq!(
            table.cells,
            if matches {
                vec![Value::File(FileId(0))]
            } else {
                vec![]
            }
        );
    }
    assert_eq!(
        union_semantic_calls.0.load(Ordering::SeqCst),
        if refused { 0 } else { 2 },
        "the singleton union schedules its one member as both inputs: two real SetRow Union semantic calls produce one distinct output row; explicit frontier refusal permits none"
    );
    assert_eq!(
        project.reads.load(Ordering::SeqCst),
        if refused { 0 } else { 1 },
        "no producer or provider is rerun for the union"
    );
    request.finish();
}
