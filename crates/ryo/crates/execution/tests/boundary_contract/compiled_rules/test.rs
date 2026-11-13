//! Independently compiled rules keep their declarations and frontiers through the owner.
use super::super::*;
use ryo_execution::inbound::use_cases::compile_rules::interface::CompileRulesUseCase as _;
use ryo_execution::inbound::use_cases::compile_rules::types::CompileRequest;
use ryo_execution::inbound::use_cases::compile_rules::use_case::CompileRulesUseCaseImpl;

/// Rule compilation over `provider`, the real handover into preparation.
fn compiling_with(provider: Arc<dyn QueryLanguagePort>) -> CompileRulesUseCaseImpl {
    CompileRulesUseCaseImpl::new(
        provider,
        Arc::new(ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake),
    )
}

/// A provider result, not a query parser or execution implementation: this
/// control exercises the real compilation handover and every subsequent owner.
struct AuthoredQueryProvider(Plan);

impl QueryLanguagePort for AuthoredQueryProvider {
    fn compile(
        &self,
        _: &QueryBundle,
        _: &[ryo_values::domain::hosted::types::HostedRelation],
        _: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<Plan, QueryLanguageError> {
        Ok(self.0.clone())
    }
    fn validate(
        &self,
        _: &QueryBundle,
        _: &[ryo_values::domain::hosted::types::HostedRelation],
        _: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<ryo_values::domain::descriptions::types::ValidateReport, QueryLanguageError> {
        Ok(ryo_values::domain::descriptions::types::ValidateReport {
            relations: vec![],
            operators: vec![],
            steps: ryo_values::domain::descriptions::types::StepCount(self.0.steps.len()),
        })
    }
}

#[test]
fn public_compiled_independent_rules_keep_exact_per_rule_source_declarations() {
    let engine = engine();
    let raw = authored_request(2);
    let compiler = compiling_with(Arc::new(AuthoredQueryProvider(
        crate::boundary_fixture::fixtures::plans_of(&raw)[0].clone(),
    )));
    let compiled = compiler
        .compile_rules(CompileRequest {
            rules: &raw.rules,
            hosted: &raw.hosted,
            vocabularies: &ryo_values::domain::descriptions::types::KindVocabularies(Vec::new()),
        })
        .expect("compile independent rules");
    let program = engine
        .prepare(PrepareRequest {
            plans: PlanSource::Compiled(compiled),
            rules: raw.rules,
            digests: raw.digests,
            hosted: raw.hosted,
            observations: ObservationIdentity::Independent,
        })
        .expect("exact compiled declarations accepted");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
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
                resident_stability: ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[
                    FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::Disabled,
                    },
                    FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::Disabled,
                    },
                ],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
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
        .compile_rules(CompileRequest {
            rules: &raw.rules,
            hosted: &raw.hosted,
            vocabularies: &ryo_values::domain::descriptions::types::KindVocabularies(Vec::new()),
        })
        .expect("compile second request");
    let mut substituted = raw.rules;
    substituted.pop();
    assert!(
        matches!(
            engine.prepare(PrepareRequest {
                plans: PlanSource::Compiled(compiled),
                rules: substituted,
                digests: vec![Digest([0; 16])],
                hosted: vec![],
                observations: ObservationIdentity::Independent
            }),
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
    use ryo_values::domain::row_texts::types::Cell as HostCell;
    for populated in [false, true] {
        let engine = engine();
        let mut input = authored_request(1);
        crate::boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
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
        crate::boundary_fixture::fixtures::plans(&mut input)[0]
            .emit
            .step = StepId(1);
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
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
        let elements = ryo_language::domain::elements::types::Elements::default();
        let read = |_| Err::<SourceRows<'_>, _>(OperatingError::NotRead);
        for delivery in [0, 1, 2] {
            let mut symbols = Symbols::default();
            let intents = [FileRuleIntent::Run {
                diagnostic: if delivery == 0 {
                    DiagnosticDelivery::Collected
                } else {
                    DiagnosticDelivery::Forwarded
                },
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            }];
            let mut file = request
                .open_file(
                    FileInput {
                        language: SourceLanguage::Recognized,
                        resident_stability: ResidentStability::Stable,
                        file: FileId(0),
                        evaluation: &[],
                        publication: &[],
                        intents: &intents,
                        source: ryo_values::domain::files::types::SourceView(b"abc"),
                        elements: &elements,
                        read: &read,
                        semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                        kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
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
            // The key partitions come in key-hash order: under the derived key hash (one `FxHasher` step) key 5 hashes to
            // 0x4b9f_b543_66d8_2d9a, below key 7's 0x9d12_ca91_8e61_d971, so the partition of key 5 comes first.
            let expected: Vec<Vec<_>> = if populated {
                [
                    [5, 1, 2],
                    [5, 1, 3],
                    [5, 2, 3],
                    [7, 9, 10],
                    [7, 9, 11],
                    [7, 10, 11],
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
    use ryo_execution::inbound::use_cases::execution::interface::ProgramPreparedExecutionPort;
    use ryo_values::domain::plan_parts::types::RepeatedRows;
    let engine = engine();
    let mut raw = authored_request(1);
    crate::boundary_fixture::fixtures::plans(&mut raw)[0]
        .steps
        .extend([
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
    crate::boundary_fixture::fixtures::plans(&mut raw)[0].frontier = vec![
        Input::Step(StepId(2)),
        Input::Step(StepId(0)),
        Input::Step(StepId(2)),
    ];
    let original = crate::boundary_fixture::fixtures::plans(&mut raw)[0].clone();
    let compiler = compiling_with(Arc::new(AuthoredQueryProvider(original.clone())));
    let compiled = compiler
        .compile_rules(CompileRequest {
            rules: &raw.rules,
            hosted: &raw.hosted,
            vocabularies: &ryo_values::domain::descriptions::types::KindVocabularies(Vec::new()),
        })
        .expect("independent compile");
    assert_eq!(
        compiled.plans(),
        [original.clone()],
        "independent preparation must not perform observation pruning/remapping"
    );
    let program = engine
        .prepare(PrepareRequest {
            plans: PlanSource::Compiled(compiled),
            rules: raw.rules,
            digests: raw.digests,
            hosted: raw.hosted,
            observations: ObservationIdentity::Independent,
        })
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
