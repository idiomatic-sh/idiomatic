//! The public protocol is the only execution entry used by these controls.
use ryo_execution::inbound::use_cases::execution::{
    interface::{ExecutionRequest, ExecutionUseCase, FileExecution},
    types::*,
    use_case::ExecutionUseCaseImpl,
};
use ryo_language::{domain::files::types::FilePath, domain::symbols::types::Symbols};
use ryo_relations::domain::relations::{
    service::{kinds, node_id},
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value},
};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input, StepId},
    plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
    program::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    rows::types::{RowCount, Table},
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
};
use std::{cell::Cell, collections::BTreeMap, sync::Arc};
fn engine() -> ExecutionUseCaseImpl {
    ExecutionUseCaseImpl::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    )
}
fn prepared(engine: &ExecutionUseCaseImpl) -> PreparedProgram {
    prepared_rules(engine, 1)
}
fn authored_request(count: usize) -> PreparedRequest {
    PreparedRequest {
        plans: vec![
            Plan {
                parameters: vec![],
                observation_inputs: vec![],
                observations: vec![],
                steps: vec![PlanStep {
                    scope: StepScope::PerFile,
                    operation: Operation::Select {
                        from: Input::Relation(Relation::Function),
                        comparisons: vec![],
                        kept: vec![ColumnIndex(0)],
                    },
                }],
                comparison_origins: vec![],
                frontier: vec![],
                emit: Emit {
                    step: StepId(0),
                    subject: ColumnIndex(0),
                    key: vec![],
                    headings: vec![],
                },
            };
            count
        ],
        bindings: vec![vec![]; count],
        rules: (0..count)
            .map(|index| RuleSpec {
                id: RuleId(format!("public-boundary-{index}").into()),
                query: QueryBundle {
                    root: QueryDocumentSource {
                        id: QuerySourceId(format!("public-boundary-{index}").into()),
                        text: "".into(),
                    },
                    modules: BTreeMap::new(),
                },
                applicability: RuleApplicability::Always,
                given: vec![],
                observations: vec![],
                spelled: RuleParameters("".into()),
                include: vec![],
                exclude: vec![],
            })
            .collect(),
        digests: (0..count).map(|index| Digest([index as u8; 16])).collect(),
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    }
}
fn prepared_rules(engine: &ExecutionUseCaseImpl, count: usize) -> PreparedProgram {
    engine
        .prepare(authored_request(count))
        .expect("valid public request")
}
fn corpus() -> Vec<CorpusInput> {
    vec![CorpusInput::Available(CorpusEntry {
        executable: ExecutablePermission::Unknown,
        path: FilePath("input.py".into()),
        size: FileSize(3),
        modified: Nanoseconds(0),
    })]
}
fn source() -> Table {
    Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(1),
        cells: kinds(Relation::Function)
            .iter()
            .map(|kind| match kind {
                CellKind::Node => Value::Node(node_id(FileId(0), Position(2))),
                CellKind::Symbol => Value::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                CellKind::File => Value::File(FileId(0)),
                CellKind::Span => Value::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                CellKind::Number => Value::Number(Integer(0)),
                CellKind::Flag => Value::Flag(Flag::Lowered),
            })
            .collect(),
    }
}
fn demand() -> RuleDemand {
    RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    }
}
#[test]
fn public_file_run_publishes_once_and_finishes() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let table = source();
    let reads = Cell::new(0);
    let read = |relation| {
        reads.set(reads.get() + 1);
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&table))
        } else {
            Err(OperatingError::NotRead)
        }
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
                demanded: &[true],
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .expect("open actual file");
    let RootOutcome::Complete(first) = file
        .rule(RuleIndex(0), demand(), &mut symbols)
        .expect("first rule")
        .emit
    else {
        panic!("complete publication required")
    };
    assert_eq!(
        first.cells,
        vec![Value::Node(node_id(FileId(0), Position(2)))]
    );
    assert_eq!(first.count, RowCount(1));
    let after_first = reads.get();
    let RootOutcome::Complete(second) = file
        .rule(RuleIndex(0), demand(), &mut symbols)
        .expect("cached rule")
        .emit
    else {
        panic!("complete cached publication required")
    };
    assert_eq!(first, second);
    assert_eq!(reads.get(), after_first);
    file.finish().expect("finish actual owner");
    request.finish();
}
#[test]
fn public_file_run_refuses_replaced_symbol_population_before_reading() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let reads = Cell::new(0);
    let read = |_| {
        reads.set(reads.get() + 1);
        Err(OperatingError::NotRead)
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
                demanded: &[true],
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .expect("open actual file");
    let mut foreign = Symbols::default();
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut foreign),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(reads.get(), 0);
}
#[test]
fn public_open_refuses_file_outside_request() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let read = |_| Err(OperatingError::NotRead);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    assert!(matches!(
        request.open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: facts::ResidentStability::Stable,
                file: FileId(9),
                evaluation: &[],
                publication: &[],
                demanded: &[true],
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None
            },
            &mut symbols
        ),
        Err(OperatingError::NotRead)
    ));
}

#[test]
fn public_skipped_rule_cannot_be_cancelled_twice_or_executed() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let table = source();
    let reads = Cell::new(0);
    let read = |relation| {
        reads.set(reads.get() + 1);
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&table))
        } else {
            Err(OperatingError::NotRead)
        }
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
                demanded: &[true],
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .expect("open actual file");
    file.skip_rule(RuleIndex(0), ApplicabilityReason::DiagnosticNotApplicable)
        .expect("cancel the real compiled root without reading facts");
    assert!(matches!(
        file.skip_rule(RuleIndex(0), ApplicabilityReason::DiagnosticNotApplicable),
        Err(OperatingError::NotRun)
    ));
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(reads.get(), 0);
    file.finish().expect("finish after cancellation");
    request.finish();
}

#[test]
fn public_skip_of_one_shared_rule_preserves_the_other_rules_reader() {
    let engine = engine();
    // Identical valid pure plans share the actual physical producer while IDs,
    // digests and logical applicability remain distinct.
    let program = prepared_rules(&engine, 2);
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let table = source();
    let reads = Cell::new(0);
    let read = |relation| {
        reads.set(reads.get() + 1);
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&table))
        } else {
            Err(OperatingError::NotRead)
        }
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
                demanded: &[true, true],
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .expect("two actual logical rules");
    file.skip_rule(RuleIndex(0), ApplicabilityReason::DiagnosticNotApplicable)
        .expect("cancel only the first logical reader");
    assert!(matches!(
        file.skip_rule(RuleIndex(0), ApplicabilityReason::DiagnosticNotApplicable),
        Err(OperatingError::NotRun)
    ));
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(
        reads.get(),
        0,
        "neither skip nor its refused repetition reads the source"
    );
    let RootOutcome::Complete(remaining) = file
        .rule(RuleIndex(1), demand(), &mut symbols)
        .expect("the other logical reader remains valid")
        .emit
    else {
        panic!("shared producer must remain available")
    };
    assert_eq!(remaining.count, RowCount(1));
    assert_eq!(
        remaining.cells,
        vec![Value::Node(node_id(FileId(0), Position(2)))]
    );
    let after_first = reads.get();
    assert!(after_first > 0);
    assert!(
        matches!(
            file.skip_rule(RuleIndex(1), ApplicabilityReason::DiagnosticNotApplicable),
            Err(OperatingError::NotRun)
        ),
        "an active reader cannot be retroactively cancelled"
    );
    let RootOutcome::Complete(repeated) = file
        .rule(RuleIndex(1), demand(), &mut symbols)
        .expect("normal repeated publication still works")
        .emit
    else {
        panic!("cached shared publication required")
    };
    assert_eq!(repeated, remaining);
    assert_eq!(reads.get(), after_first);
    file.finish()
        .expect("finish actual owner after shared skip");
    request.finish();
}

/// A provider result, not a query parser or execution implementation: this
/// control exercises the real compilation handover and every subsequent owner.
struct AuthoredQueryProvider(Plan);
impl query_language::QueryLanguagePort for AuthoredQueryProvider {
    fn compile(
        &self,
        _: &QueryBundle,
        _: &[ryo_values::domain::hosted::types::HostedRelation],
        _: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<Plan, query_language::QueryLanguageError> {
        Ok(self.0.clone())
    }
    fn validate(
        &self,
        _: &QueryBundle,
        _: &[ryo_values::domain::hosted::types::HostedRelation],
        _: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<
        ryo_values::domain::descriptions::types::ValidateReport,
        query_language::QueryLanguageError,
    > {
        Ok(ryo_values::domain::descriptions::types::ValidateReport {
            relations: vec![],
            operators: vec![],
            steps: ryo_values::domain::descriptions::types::StepCount(self.0.steps.len()),
        })
    }
}
#[test]
fn public_compiled_independent_rules_keep_exact_per_rule_source_declarations() {
    use ryo_execution::inbound::use_cases::execution::{
        interface::ExecutionCompilation, use_case::CompilationUseCaseImpl,
    };
    let engine = engine();
    let raw = authored_request(2);
    let compiler =
        CompilationUseCaseImpl::new(Arc::new(AuthoredQueryProvider(raw.plans[0].clone())));
    let compiled = compiler
        .compile(&raw.rules, &raw.hosted)
        .expect("compile independent rules");
    let program = engine
        .prepare_compiled(
            compiled,
            raw.rules,
            raw.digests,
            raw.hosted,
            ObservationIdentity::Independent,
        )
        .expect("exact compiled declarations accepted");
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let table = source();
    let read = |relation| {
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&table))
        } else {
            Err(OperatingError::NotRead)
        }
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
                demanded: &[true, true],
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .expect("open");
    for rule in [RuleIndex(0), RuleIndex(1)] {
        let RootOutcome::Complete(table) = file
            .rule(rule, demand(), &mut symbols)
            .expect("run independent rule")
            .emit
        else {
            panic!("complete root")
        };
        assert_eq!(table.cells, [Value::Node(node_id(FileId(0), Position(2)))]);
    }
    file.finish().expect("finish");
    request.finish();

    let raw = authored_request(2);
    let compiled = compiler
        .compile(&raw.rules, &raw.hosted)
        .expect("compile second request");
    let mut substituted = raw.rules;
    substituted.pop();
    assert!(
        matches!(
            engine.prepare_compiled(
                compiled,
                substituted,
                vec![Digest([0; 16])],
                vec![],
                ObservationIdentity::Independent
            ),
            Err(OperatingError::ShapeMismatch)
        ),
        "a consumer cannot relabel the two-rule provider result"
    );
}

#[test]
fn closure_runs_every_partition_and_empty_input_through_the_owner() {
    use ryo_values::domain::hosted::types::{
        HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
    };
    use ryo_values::domain::plan_parts::types::HostedNumber;
    use ryo_values::domain::records::types::Cell as HostCell;
    for populated in [false, true] {
        let engine = engine();
        let mut input = authored_request(1);
        input.plans[0].steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Hosted {
                    relation: HostedNumber(0),
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Closure {
                    edges: Input::Step(StepId(0)),
                    keys: vec![ColumnIndex(2)],
                    source: ColumnIndex(0),
                    destination: ColumnIndex(1),
                },
            },
        ];
        input.plans[0].emit.step = StepId(1);
        input.hosted = vec![HostRelation {
            name: HostName("partitioned-edges".into()),
            columns: ["from", "to", "key"]
                .into_iter()
                .map(|name| HostColumn {
                    name: HostColumnName(name.into()),
                    kind: HostKind::Number,
                })
                .collect(),
            rows: if populated {
                [[1, 2, 5], [9, 10, 7], [2, 3, 5], [10, 11, 7]]
                    .into_iter()
                    .map(|row| {
                        HostRow(
                            row.into_iter()
                                .map(|n| HostCell::Number(Integer(n)))
                                .collect(),
                        )
                    })
                    .collect()
            } else {
                vec![]
            },
        }];
        let program = engine.prepare(input).expect("closure plan");
        let corpus = corpus();
        let request = engine.begin(&program, &corpus);
        let elements = ryo_language::domain::elements::types::Elements::default();
        let read = |_| Err::<SourceRows<'_>, _>(OperatingError::NotRead);
        for delivery in [0, 1, 2] {
            let mut symbols = Symbols::default();
            let mut file = request
                .open_file(
                    FileInput {
                        language: SourceLanguage::Recognized,
                        resident_stability: facts::ResidentStability::Stable,
                        file: FileId(0),
                        evaluation: &[],
                        publication: &[],
                        demanded: &[true],
                        source: b"abc",
                        elements: &elements,
                        read: &read,
                        semantic: None,
                        kinds: None,
                    },
                    &mut symbols,
                )
                .expect("closure file");
            let mut cells = Vec::new();
            if delivery == 2 {
                let mut callbacks = 0;
                let result = file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| {
                    callbacks += 1;
                    Err(OperatingError::WrongBinding)
                });
                if populated {
                    assert!(matches!(result, Err(OperatingError::WrongBinding)));
                    assert_eq!(callbacks, 1, "stop at the first output refusal");
                } else {
                    assert!(result.is_ok());
                    assert_eq!(callbacks, 0);
                }
                file.finish().expect("release closure after sink refusal");
                continue;
            }
            if delivery == 1 {
                file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                    cells.extend_from_slice(row);
                    Ok(())
                })
                .expect("delivered closure rows");
            } else {
                let RootOutcome::Complete(table) = file
                    .rule(RuleIndex(0), demand(), &mut symbols)
                    .expect("closure run")
                    .emit
                else {
                    panic!("complete closure required")
                };
                assert_eq!(table.kinds, vec![CellKind::Number; 3]);
                assert_eq!(table.count, RowCount(if populated { 6 } else { 0 }));
                cells = table.cells;
            }
            assert_eq!(cells.len() % 3, 0, "no incomplete published row");
            let rows: Vec<_> = cells.chunks_exact(3).map(|row| row.to_vec()).collect();
            let expected: Vec<Vec<_>> = if populated {
                [
                    [7, 9, 10],
                    [7, 9, 11],
                    [7, 10, 11],
                    [5, 1, 2],
                    [5, 1, 3],
                    [5, 2, 3],
                ]
                .into_iter()
                .map(|row| row.into_iter().map(|n| Value::Number(Integer(n))).collect())
                .collect()
            } else {
                vec![]
            };
            assert_eq!(
                rows, expected,
                "every key partition, both public output forms"
            );
            file.finish().expect("closure releases the file owner");
        }
        request.finish();
    }
}

#[test]
fn independent_compilation_preserves_unused_steps_and_original_frontier_order() {
    use ryo_execution::inbound::use_cases::execution::{
        interface::{ExecutionCompilation, PreparedExecution, PreparedQueries},
        use_case::CompilationUseCaseImpl,
    };
    use ryo_values::domain::plan_parts::types::RepeatedRows;
    let engine = engine();
    let mut raw = authored_request(1);
    raw.plans[0].steps.extend([
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Relation(Relation::Function),
                columns: vec![ColumnIndex(1)],
                distinct: RepeatedRows::Kept,
            },
        },
    ]);
    raw.plans[0].frontier = vec![
        Input::Step(StepId(2)),
        Input::Step(StepId(0)),
        Input::Step(StepId(2)),
    ];
    let original = raw.plans[0].clone();
    let compiler = CompilationUseCaseImpl::new(Arc::new(AuthoredQueryProvider(original.clone())));
    let compiled = compiler
        .compile(&raw.rules, &raw.hosted)
        .expect("independent compile");
    assert_eq!(
        compiled.plans(),
        [original.clone()],
        "independent preparation must not perform observation pruning/remapping"
    );
    let program = engine
        .prepare_compiled(
            compiled,
            raw.rules,
            raw.digests,
            raw.hosted,
            ObservationIdentity::Independent,
        )
        .expect("prepare exact handover");
    assert_eq!(
        program.frontiers(RuleIndex(0)).expect("frontier metadata"),
        original.frontier
    );
    assert_eq!(
        program.emit(RuleIndex(0)).expect("emit metadata"),
        &original.emit
    );
}

#[derive(Debug, Default)]
struct ReductionAudit {
    reductions: std::sync::atomic::AtomicUsize,
    emissions: std::sync::atomic::AtomicUsize,
}
impl ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase for ReductionAudit {
    fn operators(
        &self,
        call: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::interface::OperatorAnswer,
        ryo_operators::inbound::use_cases::operators::interface::PortableOperatorFault,
    > {
        use ryo_operators::inbound::use_cases::operators::{
            interface::{OperatorCall, OperatorsUseCase},
            use_case::OperatorsUseCaseImpl,
        };
        match &call {
            OperatorCall::ReduceCount { .. } => {
                self.reductions
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            OperatorCall::Aggregate { .. } => {
                self.emissions
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            _ => {}
        }
        OperatorsUseCase::operators(&OperatorsUseCaseImpl::new(), call)
    }
}
#[test]
fn public_group_reduction_and_emission_use_the_admitted_semantic_path() {
    let semantics = Arc::new(ReductionAudit::default());
    let engine = ExecutionUseCaseImpl::new(
        semantics.clone(),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    );
    let mut input = authored_request(1);
    input.plans[0].steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::GroupCount {
            from: Input::Step(StepId(0)),
            keys: vec![],
            distinct: None,
        },
    });
    input.plans[0].emit.step = StepId(1);
    let program = engine.prepare(input).expect("prepare real count");
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let table = source();
    let read = |relation| {
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&table))
        } else {
            Err(OperatingError::NotRead)
        }
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
                demanded: &[true],
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .expect("open count file");
    let RootOutcome::Complete(table) = file
        .rule(RuleIndex(0), demand(), &mut symbols)
        .expect("count through owner")
        .emit
    else {
        panic!("complete count")
    };
    assert_eq!(table.count, RowCount(1));
    assert_eq!(table.cells, [Value::Number(Integer(1))]);
    assert_eq!(
        semantics
            .reductions
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    assert_eq!(
        semantics
            .emissions
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    file.finish().expect("finish count");
    request.finish();
}
