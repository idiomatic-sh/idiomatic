//! Real registered DAG producers must survive required consumers independently of cache admission.
use super::*;
use crate::application::file_dag::interface::DagExecution as _;
use crate::domain::file_program::service::compile_prepared;
use crate::domain::operator::contract::types::RequestOperators;
use crate::domain::replay::types::Collect;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::{kinds, to_bits},
    types::{ColumnsView, FactColumn, FileId, Integer},
};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, RepeatedRows, StepId},
    plans::types::{Emit as PlanEmit, Operation, Plan, PlanStep, StepScope},
};

fn diamond() -> Plan {
    let step = |operation| PlanStep {
        operation,
        scope: StepScope::PerFile,
    };
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps: vec![
            step(Operation::Select {
                from: Input::Relation(Relation::LanguageTarget),
                comparisons: vec![],
                kept: vec![],
            }),
            step(Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(1)],
                distinct: RepeatedRows::Kept,
            }),
            step(Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(2)],
                distinct: RepeatedRows::Kept,
            }),
            step(Operation::Union {
                left: Input::Step(StepId(1)),
                right: Input::Step(StepId(2)),
            }),
            step(Operation::Project {
                from: Input::Step(StepId(3)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            }),
        ],
        emit: PlanEmit {
            step: StepId(4),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    }
}
#[test]
fn wide_diamond_required_prefix_runs_once_with_zero_or_small_optional_budget() {
    for allowance in [0, 1, 2 * 1024 * 1024] {
        let plan = diamond();
        let operators = RequestOperators::prepare(
            std::slice::from_ref(&plan),
            &[vec![]],
            &[],
            (std::sync::Arc::new(
                ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
            ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
        )
        .unwrap();
        let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &operators));
        let [producer, left, right, root] = [0, 1, 2, 3].map(|at| graph.cuts[0][at].unwrap());
        let scope = crate::application::file_dag::testing::FileScope::new(
            graph,
            crate::application::file_dag::execution::CommonOperatorEngine::fixture_ref(),
            |_| true,
            &[Relation::LanguageTarget],
            &[],
        )
        .unwrap();
        let dag = &scope.execution;
        dag.borrow_mut().limit = allowance;
        let count = 4096usize;
        let columns = [
            FactColumn(
                (0..count)
                    .map(|n| to_bits(Value::File(FileId(n as u32))).0)
                    .collect(),
            ),
            FactColumn(vec![to_bits(Value::Number(Integer(3))).0; count]),
            FactColumn(
                (0..count)
                    .map(|n| to_bits(Value::Number(Integer(n as i64 + 10))).0)
                    .collect(),
            ),
        ];
        let read = |relation| {
            Ok(Rows::Stored(ColumnsView {
                kinds: kinds(relation),
                columns: &columns,
            }))
        };
        let mut symbols = Symbols::default();
        let mut scratch = OperatorScratch::default();
        let mut memo = vec![];
        let mut context = OperatorContext {
            semantic: None,
            prepared_matchers: None,
            prepared_kinds: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: ryo_values::domain::files::types::SourceView(b""),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        dag.begin_rule(0).unwrap();
        drop(demand(&scope, left, &read, &mut context).unwrap());
        assert!(
            dag.borrow().required[producer.0].is_some(),
            "second actual consumer still needs the wide prefix"
        );
        assert!(
            dag.borrow().required_bytes > 0,
            "required work is accounted, not called free cache"
        );
        if allowance <= 1 {
            assert!(dag.borrow().required_bytes > allowance);
        }
        drop(demand(&scope, right, &read, &mut context).unwrap());
        assert!(
            dag.borrow().required[producer.0].is_none(),
            "release at actual last local consumer"
        );
        assert_eq!(dag.borrow().executions[producer.0], 1);
        let answer = demand(&scope, root, &read, &mut context).unwrap();
        let mut expected = vec![Value::Number(Integer(3))];
        expected.extend((0..count).map(|n| Value::Number(Integer(n as i64 + 10))));
        assert_eq!(answer.cells, expected, "exact first-seen union order");
        assert_eq!(
            dag.borrow().executions[producer.0],
            1,
            "later parent access must not reevaluate ancestors"
        );
        assert_eq!(dag.borrow().executions[left.0], 1);
        assert_eq!(dag.borrow().executions[right.0], 1);
        assert!(dag.borrow().retained_bytes <= allowance);
        assert!(dag.borrow().peak_required_bytes > 0);
        if allowance <= 1 {
            assert_eq!(dag.borrow().overlap_bytes, 0);
        } else {
            assert_eq!(
                dag.borrow().overlap_bytes,
                bytes(dag.borrow().required[root.0].as_ref().unwrap()),
                "same Arc payload charged once in unique ownership"
            );
        }
        let remaining = dag.borrow().remaining.clone();
        assert_eq!(
            demand(&scope, root, &read, &mut context).unwrap().cells,
            answer.cells
        );
        assert_eq!(
            dag.borrow().remaining,
            remaining,
            "repeated successful reads never consume static edges twice"
        );
        dag.borrow_mut().finish_rule(0);
        assert_eq!(dag.borrow().required_bytes, 0);
        assert!(dag.borrow().required.iter().all(Option::is_none));
        assert!(
            dag.borrow()
                .remaining
                .iter()
                .all(|remaining| *remaining == 0)
        );
    }
}
#[test]
fn held_and_streamed_reads_of_one_dependency_use_one_invocation_pin() {
    let plan = diamond();
    let ops = RequestOperators::prepare(
        std::slice::from_ref(&plan),
        &[vec![]],
        &[],
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    )
    .unwrap();
    let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &ops));
    let child = graph.cuts[0][0].unwrap();
    let scope = crate::application::file_dag::testing::FileScope::new(
        (graph).clone(),
        crate::application::file_dag::execution::CommonOperatorEngine::fixture_ref(),
        |_| true,
        &[Relation::LanguageTarget],
        &[],
    )
    .unwrap();
    let dag = &scope.execution;
    dag.borrow_mut().limit = 0;
    dag.begin_rule(0).unwrap();
    let columns = [
        FactColumn(vec![to_bits(Value::File(FileId(1))).0]),
        FactColumn(vec![to_bits(Value::Number(Integer(3))).0]),
        FactColumn(vec![to_bits(Value::Number(Integer(10))).0]),
    ];
    let read = |relation| {
        Ok(Rows::Stored(ColumnsView {
            kinds: kinds(relation),
            columns: &columns,
        }))
    };
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut context = OperatorContext {
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let held = [OnceCell::new()];
    {
        let inputs = [NodeInput::Relation(Relation::Function)];
        let schemas = [vec![CellKind::Number]];
        let unavailable = |_| panic!("metadata must not request provider rows");
        let metadata = GraphInputs {
            owner: std::rc::Rc::clone(scope.scope.owner()),
            node: child,
            phase: StepScope::PerFile,
            region: None,
            request_source: None,
            source_kinds: &[],
            supply: &[Supply::AtHand],
            native_join_right_first: false,
            executor: scope.scope.executor(),
            execution: dag,
            inputs: &inputs,
            schemas: &schemas,
            held: &held,
            read: &unavailable,
        };
        assert_eq!(
            metadata.kinds(InputPlace(0)).unwrap(),
            &[CellKind::Number],
            "prepared custom schema is authoritative over catalog defaults"
        );
        assert!(metadata.publication(InputPlace(0)).unwrap().is_none());
    }
    let inputs = [NodeInput::Node(child)];
    let mut feed = GraphInputs {
        owner: std::rc::Rc::clone(scope.scope.owner()),
        node: child,
        phase: StepScope::PerFile,
        region: None,
        request_source: None,
        source_kinds: &[],
        supply: &[Supply::AtHand],
        native_join_right_first: false,
        executor: scope.scope.executor(),
        execution: dag,
        inputs: &inputs,
        schemas: std::slice::from_ref(graph.nodes[child.0].contract.schema.as_ref().unwrap()),
        held: &held,
        read: &read,
    };
    let mut table = Table::default();
    assert_eq!(
        crate::domain::table_model::service::count_of(
            &feed.hold(InputPlace(0), &mut context, &mut table).unwrap()
        )
        .0,
        1
    );
    let mut first = Table::default();
    feed.stream(InputPlace(0), &mut context, &mut Collect(&mut first))
        .unwrap();
    let mut second = Table::default();
    feed.stream(InputPlace(0), &mut context, &mut Collect(&mut second))
        .unwrap();
    assert_eq!(first, second);
    drop(feed);
    assert_eq!(dag.borrow().executions[child.0], 1);
    assert_eq!(
        dag.borrow().demands,
        1,
        "held and streamed reads share one primitive-local pin"
    );
    dag.borrow_mut().finish_rule(0);
    assert_eq!(dag.borrow().required_bytes, 0);
}

#[test]
fn failed_primitive_is_never_published_or_counted_as_a_successful_dependency() {
    use ryo_relations::domain::relations::types::{CellKind, Offset, Span};
    use ryo_values::domain::plan_parts::types::LiteralText;
    let plan = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps: vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::SourceMatches {
                    from: Input::Relation(Relation::Function),
                    span: ColumnIndex(0),
                    base_line: None,
                    pattern: LiteralText("(".into()),
                    capture: None,
                    capture_text: false,
                    capture_line: false,
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: Input::Step(StepId(0)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
            },
        ],
        emit: PlanEmit {
            step: StepId(1),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    };
    let ops = RequestOperators::prepare_with_schemas(
        std::slice::from_ref(&plan),
        &[vec![]],
        &[],
        &|_| &[CellKind::Span],
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    )
    .unwrap();
    let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &ops));
    let node = graph.cuts[0][0].unwrap();
    let scope = crate::application::file_dag::testing::FileScope::new(
        graph,
        crate::application::file_dag::execution::CommonOperatorEngine::fixture_ref(),
        |_| true,
        &[Relation::Function],
        &[],
    )
    .unwrap();
    let dag = &scope.execution;
    dag.borrow_mut().limit = 0;
    dag.begin_rule(0).unwrap();
    let table = Table {
        kinds: vec![CellKind::Span],
        cells: vec![Value::Span(Span {
            start: Offset(0),
            end: Offset(1),
        })],
        count: RowCount(1),
    };
    let reads = std::cell::Cell::new(0);
    let read = |_| {
        reads.set(reads.get() + 1);
        Ok(Rows::Computed(&table))
    };
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut context = OperatorContext {
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b"x"),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    for _ in 0..2 {
        assert!(matches!(
            demand(&scope, node, &read, &mut context),
            Err(OperatingError::InvalidRegex { .. })
        ));
        assert!(dag.borrow().required[node.0].is_none());
        assert!(dag.borrow().ready[node.0].is_none());
        assert_eq!(dag.borrow().executions[node.0], 0);
        assert!(!dag.borrow().visiting[node.0]);
    }
    assert_eq!(reads.get(), 2, "failed results are never reused");
    dag.borrow_mut().finish_rule(0);
    assert_eq!(dag.borrow().required_bytes, 0);
    assert_eq!(dag.borrow().retained_bytes, 0);
}

fn demand<'f>(
    scope: &crate::application::file_dag::testing::FileScope<'_>,
    node: crate::domain::file_program::types::NodeId,
    read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    context: &mut OperatorContext<'_>,
) -> Result<Arc<Table>, OperatingError> {
    let published = scope
        .execution
        .demand(node, read, scope.scope.executor(), context)?;
    let rows = scope.scope.owner().resolve(published.as_ref(), read)?;
    let mut table = Table::default();
    crate::domain::table_model::service::materialize(&rows, &mut table, &mut Vec::new());
    Ok(Arc::new(table))
}
