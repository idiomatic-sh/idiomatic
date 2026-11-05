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
        _: &[ryo_values::domain::observations::types::BoundObservationSchema],
    ) -> Result<Plan, query_language::QueryLanguageError> {
        Ok(self.0.clone())
    }
    fn validate(
        &self,
        _: &QueryBundle,
        _: &[ryo_values::domain::hosted::types::HostedRelation],
        _: &[ryo_values::domain::observations::types::BoundObservationSchema],
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
