//! A compiled project emit defers at file scope while real file frontiers remain available.
use crate::boundary_fixture::{authored_request, corpus, demand, engine, source};
use ryo_execution::inbound::use_cases::execution::interface::{ProjectExecution, ProjectStage};
use ryo_execution::inbound::use_cases::execution::{
    interface::{ExecutionRequest, ExecutionUseCase, FileExecution},
    types::*,
    use_case::ExecutionUseCaseImpl,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Relation, Value},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId},
    plans::types::{Operation, PlanStep, StepScope},
    rows::types::RowCount,
};
use ryo_values::domain::{
    plan_parts::types::{InputNumber, JoinColumn, Side},
    stored_facts::types::Bucket,
};
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
mod applicability;
#[cfg(test)]
mod consumers;
#[cfg(test)]
mod declared_empty;
mod restoration;
mod source_facts;
use self::source_facts::fixtures::{CapturedFacts, ProjectFacts};

fn mixed_program(engine: &ExecutionUseCaseImpl, relation: Relation) -> PreparedProgram {
    let mut input = authored_request(1);
    let file_column = kinds(Relation::Function)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .unwrap();
    input.plans[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(u32::try_from(file_column).unwrap())],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::Relation(relation),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
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
    ];
    input.plans[0].frontier = vec![Input::Step(StepId(0)), Input::Relation(relation)];
    input.plans[0].emit.step = StepId(2);
    engine
        .prepare(input)
        .expect("compile actual mixed-phase request")
}

#[test]
fn project_relations_defer_at_file_scope_and_join_real_captured_rows_once() {
    for relation in [
        Relation::ProjectNamespaceMembership,
        Relation::ModuleIdentity,
        Relation::ManifestDiagnostic,
    ] {
        let engine = engine();
        let prepared = mixed_program(&engine, relation);
        let corpus = corpus();
        let request = engine.begin(&prepared, &corpus);
        let table = source();
        let file_reads = Cell::new(0);
        let read = |actual| {
            assert_eq!(
                actual,
                Relation::Function,
                "project facts must never be read by the file owner"
            );
            file_reads.set(file_reads.get() + 1);
            Ok(SourceRows::Computed(&table))
        };
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut symbols = Symbols::default();
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: facts::ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &[FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Forwarded,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::DiagnosticAndObservations,
                    }],
                    source: b"abc",
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .expect("open actual file owner");
        let mut emitted = vec![];
        assert!(matches!(
            file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                emitted.extend_from_slice(row);
                Ok(())
            })
            .unwrap(),
            RowEmission::Deferred {
                phase: StepScope::CrossFile
            }
        ));
        assert!(emitted.is_empty());
        assert_eq!(file_reads.get(), 0);
        assert!(
            file.counts(RuleIndex(0))
                .unwrap()
                .fed
                .iter()
                .all(|count| count.0 == [0; 3])
        );
        file.finish()
            .expect("finish separately declared deferred forward root");
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: facts::ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &[FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::DiagnosticAndObservations,
                    }],
                    source: b"abc",
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .expect("open actual file owner");
        assert!(matches!(
            file.rule(RuleIndex(0), demand(), &mut symbols)
                .unwrap()
                .emit,
            RootOutcome::Deferred {
                phase: StepScope::CrossFile
            }
        ));
        assert_eq!(file_reads.get(), 0);
        assert!(
            file.counts(RuleIndex(0))
                .unwrap()
                .fed
                .iter()
                .all(|count| count.0 == [0; 3])
        );
        let mut arena = vec![];
        let mut places = vec![];
        let mut words = Words::default();
        let mut captures = file
            .capture_frontiers(
                RuleIndex(0),
                Bucket(0),
                &mut symbols,
                CaptureContext {
                    worker: WorkerNumber(0),
                    arena: &mut arena,
                    places: &mut places,
                    words: &mut words,
                },
            )
            .expect("file frontier still executes");
        assert_eq!(
            captures.len(),
            1,
            "project-owned frontier is not captured as a file input"
        );
        assert_eq!(captures[0].input, InputNumber(0));
        let capture = captures
            .pop()
            .unwrap()
            .outcome
            .expect("actual file frontier");
        // Admission may borrow a resident view more than once. The production
        // guarantee is one source traversal and no source access during finish.
        let resolved = file_reads.get();
        assert!(resolved > 0);
        assert_eq!(file.walk_counts().passes.0, 1);
        assert_eq!(file.walk_counts().source_rows.0, table.count.0 as u64);
        assert_eq!(file.counts(RuleIndex(0)).unwrap().fed[1].0, [0; 3]);
        file.finish()
            .expect("file owner finishes without demanding project sources");
        assert_eq!(
            file_reads.get(),
            resolved,
            "finish cannot demand the source again"
        );
        let stored = CapturedFacts {
            captures: vec![(FileId(0), capture)],
            reads: AtomicUsize::new(0),
        };
        let project = ProjectFacts::new(relation);
        let covered = [Covered {
            file: FileId(0),
            rules: vec![RuleIndex(0)],
            partial: vec![],
        }];
        let phase = request.project_phase().unwrap();
        let scope = phase
            .project_scope(RuleIndex(0), &corpus, &covered, &[])
            .unwrap();
        let project_symbols = Symbols::default();
        let mut symbols = Symbols::default();
        let ordinary = request
            .prepare_frontiers(
                FrontierRequest {
                    demand: FrontierDemand::Diagnostic,
                    scope: &scope,
                    rule: RuleIndex(0),
                    first: Bucket(0),
                    facts: &stored,
                    project: &project,
                    project_symbols: &project_symbols,
                },
                &mut symbols,
            )
            .expect("admit real file frontier and project facts");
        assert_eq!(stored.reads.load(Ordering::SeqCst), 1);
        assert_eq!(project.reads.load(Ordering::SeqCst), 1);
        let partials = Partials::default();
        let mut run = request
            .open_project(
                ProjectInput {
                    scope: &scope,
                    evaluation: &partials,
                    publication: &partials,
                    ordinary,
                    bound: &[],
                    arenas: &[],
                },
                &mut symbols,
            )
            .unwrap();
        let outcome = run
            .rule(RuleIndex(0), demand(), &mut symbols)
            .expect("project emit runs at project scope");
        let RootOutcome::Complete(table) = outcome.emit else {
            panic!("one complete project diagnostic table")
        };
        assert_eq!(table.kinds, [CellKind::File]);
        assert_eq!(table.count, RowCount(1));
        assert_eq!(table.cells, [Value::File(FileId(0))]);
        let counts = outcome
            .counts
            .fed
            .iter()
            .map(|count| count.0)
            .collect::<Vec<_>>();
        let again = run.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
        assert_eq!(
            again
                .counts
                .fed
                .iter()
                .map(|count| count.0)
                .collect::<Vec<_>>(),
            counts,
            "one cached project invocation, with no second semantic evaluation"
        );
        run.finish();
        drop(scope);
        phase.finish().expect("release project publications");
        assert_eq!(project.reads.load(Ordering::SeqCst), 1);
        request.finish();
        assert_eq!(
            file_reads.get(),
            resolved,
            "project work cannot reread the file facts after their captured frontier"
        );
    }
}

mod negative;

/// Migration control for the old pipeline proof's invalid schema and arbitrary cross-file hashes.
/// Every grouped name below is obtained from a real captured Function population.
#[test]
fn project_grouping_reads_only_actual_captured_function_names() {
    use ryo_language::domain::symbols::{service::hash_of, types::ByteSlice};
    use ryo_relations::domain::relations::types::Integer;
    use ryo_values::domain::stored_facts::types::ArenaBytes;
    let engine = engine();
    let mut input = authored_request(1);
    input.plans[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(1)],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::GroupCount {
                from: Input::Step(StepId(0)),
                keys: vec![ColumnIndex(0)],
                distinct: None,
            },
        },
    ];
    input.plans[0].frontier = vec![Input::Step(StepId(0))];
    input.plans[0].emit.step = StepId(1);
    let program = engine
        .prepare(input)
        .expect("prepare actual mixed-phase count");
    let mut corpus = corpus();
    if let ryo_values::domain::files::types::CorpusInput::Available(entry) = &mut corpus[0] {
        entry.size = ryo_values::domain::files::types::FileSize(9);
    }
    let mut second = corpus[0].clone();
    if let ryo_values::domain::files::types::CorpusInput::Available(entry) = &mut second {
        entry.path = ryo_language::domain::files::types::FilePath("second.py".into());
    }
    corpus.push(second);
    let request = engine.begin(&program, &corpus);
    let mut arena = vec![];
    let mut places = vec![];
    let mut words = Words::default();
    let mut captured_files = vec![];
    let main_hash = hash_of(ByteSlice(b"main"));
    let other_hash = hash_of(ByteSlice(b"other"));
    for file_id in [FileId(0), FileId(1)] {
        let mut symbols = Symbols::default();
        let mut table = crate::boundary_fixture::named_functions(&mut symbols);
        let width = table.kinds.len();
        if file_id == FileId(0) {
            table.cells.truncate(width);
            table.count = RowCount(1);
        } else {
            table.cells.drain(..width);
            table.count = RowCount(2);
        }
        for (index, row) in table.cells.chunks_exact_mut(width).enumerate() {
            row[0] = Value::Node(ryo_relations::domain::relations::service::node_id(
                file_id,
                ryo_relations::domain::relations::types::Position(index as u32),
            ));
            row[2] = Value::File(file_id);
        }
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            Ok(SourceRows::Computed(&table))
        };
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: facts::ResidentStability::Stable,
                    file: file_id,
                    evaluation: &[],
                    publication: &[],
                    intents: &[FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::DiagnosticAndObservations,
                    }],
                    source: b"mainother",
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .expect("open actual file");
        assert!(matches!(
            file.rule(RuleIndex(0), demand(), &mut symbols)
                .unwrap()
                .emit,
            RootOutcome::Deferred {
                phase: StepScope::CrossFile
            }
        ));

        let mut captures = file
            .capture_frontiers(
                RuleIndex(0),
                Bucket(0),
                &mut symbols,
                CaptureContext {
                    worker: WorkerNumber(0),
                    arena: &mut arena,
                    places: &mut places,
                    words: &mut words,
                },
            )
            .expect("capture actual names");
        assert_eq!(captures.len(), 1);
        let capture = captures.pop().unwrap().outcome.expect("complete capture");
        file.finish().expect("release file owner");
        captured_files.push((file_id, capture));
    }
    let stored = CapturedFacts {
        captures: captured_files,
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let covered = [FileId(0), FileId(1)].map(|file| Covered {
        file,
        rules: vec![RuleIndex(0)],
        partial: vec![],
    });
    let phase = request.project_phase().unwrap();
    let scope = phase
        .project_scope(RuleIndex(0), &corpus, &covered, &[])
        .unwrap();
    let project_symbols = Symbols::default();
    let mut symbols = Symbols::default();
    let ordinary = request
        .prepare_frontiers(
            FrontierRequest {
                demand: FrontierDemand::Diagnostic,
                scope: &scope,
                rule: RuleIndex(0),
                first: Bucket(0),
                facts: &stored,
                project: &project,
                project_symbols: &project_symbols,
            },
            &mut symbols,
        )
        .expect("admit exact persisted frame");
    assert_eq!(
        project.reads.load(Ordering::SeqCst),
        0,
        "no unrelated project facts"
    );
    assert_eq!(stored.reads.load(Ordering::SeqCst), 1);
    let partials = Partials::default();
    let arenas = [ArenaBytes(arena)];
    let mut run = request
        .open_project(
            ProjectInput {
                scope: &scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &[],
                arenas: &arenas,
            },
            &mut symbols,
        )
        .unwrap();
    let RootOutcome::Complete(result) =
        run.rule(RuleIndex(0), demand(), &mut symbols).unwrap().emit
    else {
        panic!("project count must complete")
    };
    assert_eq!(result.count, RowCount(2));
    assert_eq!(result.kinds, [CellKind::Symbol, CellKind::Number]);
    let mut actual = result
        .cells
        .chunks_exact(2)
        .map(|row| {
            let [Value::Name(name), Value::Number(Integer(count))] = row else {
                panic!("name and count")
            };
            (name.hash, *count)
        })
        .collect::<Vec<_>>();
    let mut expected = vec![(main_hash, 2), (other_hash, 1)];
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
    run.finish();
    drop(scope);
    phase.finish().unwrap();
    request.finish();
}

/// The former per-file, missing-input and shared-pool scenarios use one prepared program.
#[test]
fn named_function_selection_replays_without_running_project_steps_or_hiding_absence() {
    use ryo_relations::domain::relations::{service::node_id, types::Position};
    use ryo_values::domain::plan_parts::types::{
        Comparison, LiteralText, Operand, QueryLiteral, Test,
    };
    for absent in [false, true] {
        let engine = engine();
        let mut input = authored_request(2);
        let selected = PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
                    origin: None,
                }],
                kept: vec![
                    ColumnIndex(0),
                    ColumnIndex(1),
                    ColumnIndex(2),
                    ColumnIndex(3),
                ],
            },
        };
        let projected = PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
            },
        };
        input.plans[0].steps = vec![selected.clone(), projected.clone()];
        input.plans[0].emit.step = StepId(1);
        input.plans[1].steps = vec![
            selected,
            projected,
            PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::GroupCount {
                    from: Input::Step(StepId(1)),
                    keys: vec![ColumnIndex(0)],
                    distinct: None,
                },
            },
        ];
        input.plans[1].frontier = vec![Input::Step(StepId(1))];
        input.plans[1].emit.step = StepId(2);
        let program = engine.prepare(input).expect("compile both plans together");
        let mut corpus = corpus();
        if let ryo_values::domain::files::types::CorpusInput::Available(entry) = &mut corpus[0] {
            entry.size = ryo_values::domain::files::types::FileSize(9);
        }
        let request = engine.begin(&program, &corpus);
        let mut symbols = Symbols::default();
        let table = crate::boundary_fixture::named_functions(&mut symbols);
        let reads = Cell::new(0);
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            reads.set(reads.get() + 1);
            if absent {
                Err(OperatingError::NotRead)
            } else {
                Ok(SourceRows::Computed(&table))
            }
        };
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: facts::ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &[
                        FileRuleIntent::Run {
                            diagnostic: DiagnosticDelivery::Collected,
                            observations: ObservationDemand::NotRequired,
                            capture: CaptureIntent::DiagnosticAndObservations,
                        },
                        FileRuleIntent::Run {
                            diagnostic: DiagnosticDelivery::Collected,
                            observations: ObservationDemand::NotRequired,
                            capture: CaptureIntent::DiagnosticAndObservations,
                        },
                    ],
                    source: b"mainother",
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .unwrap();
        let outcome = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
        if absent {
            assert!(matches!(
                outcome.emit,
                RootOutcome::Failed(OperatingError::NotRead)
            ));
        } else {
            let RootOutcome::Complete(first) = outcome.emit else {
                panic!("matching declarations")
            };
            assert_eq!(first.count, RowCount(2));
            assert_eq!(
                first.cells,
                [
                    Value::Node(node_id(FileId(0), Position(0))),
                    Value::Node(node_id(FileId(0), Position(1)))
                ]
            );
            let after_first = reads.get();
            assert!(matches!(
                file.rule(RuleIndex(1), demand(), &mut symbols)
                    .unwrap()
                    .emit,
                RootOutcome::Deferred {
                    phase: StepScope::CrossFile
                }
            ));
            let mut arena = vec![];
            let mut places = vec![];
            let mut words = Words::default();
            let mut captured = file
                .capture_frontiers(
                    RuleIndex(1),
                    Bucket(0),
                    &mut symbols,
                    CaptureContext {
                        worker: WorkerNumber(0),
                        arena: &mut arena,
                        places: &mut places,
                        words: &mut words,
                    },
                )
                .expect("project rule retains its file frontier");
            assert_eq!(captured.len(), 1);
            let capture = captured.pop().unwrap().outcome.expect("complete frontier");
            let decoded = ryo_values::domain::table_words::service::decode_complete(
                &capture.words,
                &mut vec![],
            )
            .unwrap();
            assert_eq!(
                decoded, first,
                "captured rows match the actual file selection"
            );
            let RootOutcome::Complete(replayed) = file
                .rule(RuleIndex(0), demand(), &mut symbols)
                .unwrap()
                .emit
            else {
                panic!("repeat file rule")
            };
            assert_eq!(first, replayed);
            assert_eq!(
                reads.get(),
                after_first,
                "deferred and replayed roots do not reread source"
            );
        }
        file.finish().expect("finish actual file session");
        request.finish();
    }
}

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
    use ryo_execution::inbound::use_cases::execution::interface::{
        ObservationExecution, ObservationPublication, PreparedExecution,
    };
    use ryo_values::domain::observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingId, ObservationBindingName,
        ObservationColumn, ObservationColumnName, ObservationIndex, ObservationName,
        ObservationProducer, ObservationReadinessLayout, PlanObservation, ProducerReadiness,
    };
    use ryo_values::domain::records::types::{CrossPublicationTransfer, PackedObservedRoot};
    use ryo_values::domain::stored_facts::types::{ArenaBytes, Digest, KeptWords};
    use ryo_values::domain::table_words::{
        service::{encode_table, pack},
        types::WordSlice,
    };

    use ryo_operators::inbound::use_cases::operators::{
        interface::{
            OperatorAnswer, OperatorCall, OperatorsUseCase, PortableOperatorFault, SetInput,
            SetKind,
        },
        use_case::OperatorsUseCaseImpl,
    };
    #[derive(Debug, Default)]
    struct UnionSemanticCalls(AtomicUsize);
    impl OperatorsUseCase for UnionSemanticCalls {
        fn operators(
            &self,
            call: OperatorCall<'_>,
        ) -> Result<OperatorAnswer, PortableOperatorFault> {
            if matches!(
                &call,
                OperatorCall::SetRow {
                    input: SetInput::Values {
                        kind: SetKind::Union,
                        ..
                    }
                } | OperatorCall::SetRow {
                    input: SetInput::Positions {
                        kind: SetKind::Union,
                        ..
                    }
                }
            ) {
                let _previous = self.0.fetch_add(1, Ordering::SeqCst);
            }
            OperatorsUseCaseImpl::new().operators(call)
        }
    }
    let union_semantic_calls = Arc::new(UnionSemanticCalls::default());
    let engine = ExecutionUseCaseImpl::new(
        union_semantic_calls.clone(),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
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
    authored.plans[0].steps = vec![PlanStep {
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
        authored.plans[0].steps.push(PlanStep {
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
    authored.plans[0].observations.push(PlanObservation {
        name: ObservationName("files".into()),
        source: Input::Step(StepId(if selected_text.is_some() { 1 } else { 0 })),
        columns: columns.clone(),
        scope: StepScope::CrossFile,
    });
    authored.plans[1].steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: Input::BoundObservation(ObservationBindingId(0)),
            comparisons: vec![],
            kept: vec![],
        },
    }];
    authored.plans[1]
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
                ryo_values::domain::records::types::Cell::Text(selected.into()),
            ])],
        });
        authored.plans[1].steps.extend([
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
        authored.plans[1].emit.step = StepId(2);
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
    let request = engine.begin(&prepared, &corpus);
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
    let mut stage = request.observations(&mut arenas).unwrap();
    let mut symbols = Symbols::default();
    let producer_scope = stage
        .project_scope(RuleIndex(0), &corpus, &covered, &[])
        .unwrap();
    let slot = stage.reserve(RuleIndex(0)).unwrap();
    let ordinary = request
        .prepare_frontiers(
            FrontierRequest {
                demand: FrontierDemand::Diagnostic,
                scope: &producer_scope,
                rule: RuleIndex(0),
                first: Bucket(0),
                facts: &stored,
                project: &project,
                project_symbols: &project_symbols,
            },
            &mut symbols,
        )
        .unwrap();
    let partials = Partials::default();
    let mut run = request
        .open_project(
            ProjectInput {
                scope: &producer_scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &[],
                arenas: stage.arenas(),
            },
            &mut symbols,
        )
        .unwrap();
    let mut result = run
        .rule(
            RuleIndex(0),
            RuleDemand {
                emit: EmitDemand::Disabled,
                observations: ObservationDemand::Required,
            },
            &mut symbols,
        )
        .unwrap();
    assert!(matches!(result.emit, RootOutcome::NoQuery));
    let (output, observed) = result.observations.pop().unwrap();
    assert_eq!(output, ObservationIndex(0));
    let ProjectObservation::Complete {
        mut table,
        publication,
    } = observed
    else {
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
    run.finish();
    let PublicationSlot::Reserved(worker) = slot else {
        panic!("real reserved publication slot")
    };
    let mut words = Words::default();
    let mut arena = ArenaBytes::default();
    let mut places = vec![];
    ryo_execution::inbound::use_cases::publication::interface::PublicationUseCase::publication(
        &ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new(),
        ryo_execution::inbound::use_cases::publication::types::PublicationInput {
            table: &mut table,
            symbols: &mut symbols,
            inherited: stage.arenas(),
            worker,
            arena: &mut arena,
            places: &mut places,
        },
    )
    .expect("normal publication normalization and text ownership");
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
                    arenas: if selected_text.is_some() {
                        vec![worker]
                    } else {
                        vec![]
                    },
                }],
            },
            vec![(output, publication)],
        )
        .unwrap();
    stage.finish(RuleIndex(0)).unwrap();
    let consumer_scope = stage
        .project_scope(RuleIndex(1), &corpus, &covered, &[])
        .unwrap();
    let bound = stage.bindings(RuleIndex(1)).unwrap();
    assert_eq!(
        bound.len(),
        1,
        "the real stage supplies members, not a whole-union packet"
    );
    let reads = project.reads.load(Ordering::SeqCst);
    let ready = [false];
    let ordinary = request
        .prepare_frontiers(
            FrontierRequest {
                demand: if refused {
                    FrontierDemand::Observations { ready: &ready }
                } else {
                    FrontierDemand::Diagnostic
                },
                scope: &consumer_scope,
                rule: RuleIndex(1),
                first: Bucket(1),
                facts: &stored,
                project: &project,
                project_symbols: &project_symbols,
            },
            &mut symbols,
        )
        .unwrap();
    assert_eq!(
        stored.reads.load(Ordering::SeqCst),
        0,
        "no union bytes were supplied by a file frontier"
    );
    let mut run = request
        .open_project(
            ProjectInput {
                scope: &consumer_scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &bound,
                arenas: stage.arenas(),
            },
            &mut symbols,
        )
        .unwrap();
    assert_eq!(
        union_semantic_calls.0.load(Ordering::SeqCst),
        0,
        "preparing member handles and an ordinary frontier does not invoke Union semantics"
    );
    let result = run.rule(RuleIndex(1), demand(), &mut symbols).unwrap();
    if refused {
        assert!(
            matches!(result.emit, RootOutcome::Unknown(OperatingError::ObservationIncomplete { ref reason })
            if reason == "a producer feeding this retained frontier is unavailable")
        );
        assert!(result.counts.fed.iter().all(|count| count.0 == [0; 3]));
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
        reads,
        "no producer or provider is rerun for the union"
    );
    run.finish();
    drop(bound);
    stage.finish(RuleIndex(1)).unwrap();
    drop(producer_scope);
    drop(consumer_scope);
    stage.finish_stage().unwrap();
    request.finish();
}

#[test]
fn project_text_identity_does_not_depend_on_inherited_arenas() {
    use ryo_values::domain::hosted::types::{
        HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
    };
    use ryo_values::domain::plan_parts::types::{HostedNumber, LiteralText, Operand, QueryLiteral};
    use ryo_values::domain::records::types::Cell;
    use ryo_values::domain::stored_facts::types::ArenaBytes;

    for use_parameter_rows in [false, true] {
        for arenas in [vec![], vec![ArenaBytes::default()]] {
            for selected in ["F401", "F822"] {
                let engine = engine();
                let mut authored = authored_request(1);
                authored.hosted.push(HostRelation {
                    name: HostName("selected".into()),
                    columns: vec![HostColumn {
                        name: HostColumnName("code".into()),
                        kind: HostKind::Text,
                    }],
                    rows: vec![HostRow(vec![Cell::Text(selected.into())])],
                });
                authored.plans[0].steps = vec![
                    PlanStep {
                        scope: StepScope::CrossFile,
                        operation: Operation::Select {
                            from: Input::Relation(Relation::ModuleIdentity),
                            comparisons: vec![],
                            kept: vec![ColumnIndex(0)],
                        },
                    },
                    PlanStep {
                        scope: StepScope::CrossFile,
                        operation: Operation::Extend {
                            from: Input::Step(StepId(0)),
                            values: vec![Operand::Literal(QueryLiteral::Text(LiteralText(
                                "F401".into(),
                            )))],
                        },
                    },
                    PlanStep {
                        scope: StepScope::CrossFile,
                        operation: Operation::Hosted {
                            relation: HostedNumber(0),
                        },
                    },
                    PlanStep {
                        scope: StepScope::CrossFile,
                        operation: Operation::Join {
                            left: Input::Step(StepId(1)),
                            right: Input::Step(StepId(2)),
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
                ];
                if use_parameter_rows {
                    use ryo_values::domain::plan_parts::types::{
                        ParameterId, ParameterName, TableKey,
                    };
                    use ryo_values::domain::plans::types::{ParameterType, PlanParameter};
                    use ryo_values::domain::query_bindings::types::{
                        BindingCell, BindingTable, ParameterBinding,
                    };
                    authored.plans[0].parameters = vec![PlanParameter {
                        name: ParameterName("codes".into()),
                        kind: ParameterType::Tables,
                    }];
                    authored.bindings[0] =
                        vec![ParameterBinding::Tables(vec![BindingTable(vec![
                            BindingCell {
                                key: LiteralText("code".into()),
                                value: LiteralText(selected.into()),
                            },
                        ])])];
                    authored.plans[0].steps[2].operation = Operation::Rows {
                        parameter: ParameterId(0),
                        keys: vec![TableKey("code".into())],
                    };
                }
                authored.plans[0].frontier = vec![Input::Relation(Relation::ModuleIdentity)];
                authored.plans[0].emit.step = StepId(3);
                let prepared = engine.prepare(authored).unwrap();
                let corpus = corpus();
                let request = engine.begin(&prepared, &corpus);
                let covered = [Covered {
                    file: FileId(0),
                    rules: vec![RuleIndex(0)],
                    partial: vec![],
                }];
                let phase = request.project_phase().unwrap();
                let scope = phase
                    .project_scope(RuleIndex(0), &corpus, &covered, &[])
                    .unwrap();
                let stored = CapturedFacts {
                    captures: vec![],
                    reads: AtomicUsize::new(0),
                };
                let project = ProjectFacts::new(Relation::ModuleIdentity);
                let project_symbols = Symbols::default();
                let mut symbols = Symbols::default();
                let ordinary = request
                    .prepare_frontiers(
                        FrontierRequest {
                            demand: FrontierDemand::Diagnostic,
                            scope: &scope,
                            rule: RuleIndex(0),
                            first: Bucket(0),
                            facts: &stored,
                            project: &project,
                            project_symbols: &project_symbols,
                        },
                        &mut symbols,
                    )
                    .unwrap();
                let partials = Partials::default();
                let mut run = request
                    .open_project(
                        ProjectInput {
                            scope: &scope,
                            evaluation: &partials,
                            publication: &partials,
                            ordinary,
                            bound: &[],
                            arenas: &arenas,
                        },
                        &mut symbols,
                    )
                    .unwrap();
                let result = run.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
                let RootOutcome::Complete(table) = result.emit else {
                    panic!("complete project join")
                };
                assert_eq!(table.kinds, [CellKind::File]);
                assert_eq!(table.count, RowCount(usize::from(selected == "F401")));
                assert_eq!(
                    table.cells,
                    if selected == "F401" {
                        vec![Value::File(FileId(0))]
                    } else {
                        vec![]
                    }
                );
                run.finish();
            }
        }
    }
}
