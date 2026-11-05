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
    engine
        .prepare(PreparedRequest {
            plans: vec![Plan {
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
            }],
            bindings: vec![vec![]],
            rules: vec![RuleSpec {
                id: RuleId("public-boundary".into()),
                query: QueryBundle {
                    root: QueryDocumentSource {
                        id: QuerySourceId("public-boundary".into()),
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
            }],
            digests: vec![Digest([0; 16])],
            hosted: vec![],
            observations: ObservationIdentity::Independent,
        })
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
