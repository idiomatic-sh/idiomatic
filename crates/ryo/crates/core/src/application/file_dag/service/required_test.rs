//! Real registered DAG producers must survive required consumers independently of cache admission.
use super::*;
use crate::domain::file_program::service::compile_prepared;
use crate::domain::operator::contract::types::RequestOperators;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::{
    relations::{
        service::{kinds, to_bits},
        types::{ColumnsView, FactColumn, FileId, Integer},
    },
    symbols::types::Symbols,
};
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
        let operators =
            RequestOperators::prepare(std::slice::from_ref(&plan), &[vec![]], &[]).unwrap();
        let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &operators));
        let [producer, left, right, root] = [0, 1, 2, 3].map(|at| graph.cuts[0][at].unwrap());
        let mut dag = FileDagExecution::new(graph, |_| true, &[Relation::LanguageTarget]);
        dag.limit = allowance;
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
            prepared_matchers: None,
            prepared_kinds: None,
            stored_indexes: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: crate::domain::table_texts::types::SourceView(b""),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        dag.begin_rule(0);
        drop(dag.demand(left, &read, &mut context).unwrap());
        assert!(
            dag.required[producer.0].is_some(),
            "second actual consumer still needs the wide prefix"
        );
        assert!(
            dag.required_bytes > 0,
            "required work is accounted, not called free cache"
        );
        if allowance <= 1 {
            assert!(dag.required_bytes > allowance);
        }
        drop(dag.demand(right, &read, &mut context).unwrap());
        assert!(
            dag.required[producer.0].is_none(),
            "release at actual last local consumer"
        );
        assert_eq!(dag.executions[producer.0], 1);
        let answer = dag.demand(root, &read, &mut context).unwrap();
        let mut expected = vec![Value::Number(Integer(3))];
        expected.extend((0..count).map(|n| Value::Number(Integer(n as i64 + 10))));
        assert_eq!(answer.cells, expected, "exact first-seen union order");
        assert_eq!(
            dag.executions[producer.0], 1,
            "later parent access must not reevaluate ancestors"
        );
        assert_eq!(dag.executions[left.0], 1);
        assert_eq!(dag.executions[right.0], 1);
        assert!(dag.retained_bytes <= allowance);
        assert!(dag.peak_required_bytes > 0);
        if allowance <= 1 {
            assert_eq!(dag.overlap_bytes, 0);
        } else {
            assert_eq!(
                dag.overlap_bytes,
                bytes(&answer),
                "same Arc payload charged once in unique ownership"
            );
        }
        let remaining = dag.remaining.clone();
        assert_eq!(
            dag.demand(root, &read, &mut context).unwrap().cells,
            answer.cells
        );
        assert_eq!(
            dag.remaining, remaining,
            "repeated successful reads never consume static edges twice"
        );
        dag.finish_rule(0);
        assert_eq!(dag.required_bytes, 0);
        assert!(dag.required.iter().all(Option::is_none));
        assert!(dag.remaining.iter().all(|remaining| *remaining == 0));
    }
}
#[test]
fn held_and_streamed_reads_of_one_dependency_use_one_invocation_pin() {
    let plan = diamond();
    let ops = RequestOperators::prepare(std::slice::from_ref(&plan), &[vec![]], &[]).unwrap();
    let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &ops));
    let child = graph.cuts[0][0].unwrap();
    let mut dag = FileDagExecution::new(Arc::clone(&graph), |_| true, &[Relation::LanguageTarget]);
    dag.limit = 0;
    dag.begin_rule(0);
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
        prepared_matchers: None,
        prepared_kinds: None,
        stored_indexes: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let held = [OnceCell::new()];
    let inputs = [NodeInput::Node(child)];
    let mut feed = GraphInputs {
        execution: &mut dag,
        graph: &graph,
        inputs: &inputs,
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
    assert_eq!(dag.executions[child.0], 1);
    assert_eq!(
        dag.demands, 1,
        "held and streamed reads share one primitive-local pin"
    );
    dag.finish_rule(0);
    assert_eq!(dag.required_bytes, 0);
}

#[test]
fn failed_primitive_is_never_published_or_counted_as_a_successful_dependency() {
    use ryo_language::domain::relations::types::{CellKind, Offset, Span};
    use ryo_values::domain::plan_parts::types::LiteralText;
    let plan = Plan {
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
    )
    .unwrap();
    let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &ops));
    let node = graph.cuts[0][0].unwrap();
    let mut dag = FileDagExecution::new(graph, |_| true, &[Relation::Function]);
    dag.limit = 0;
    dag.begin_rule(0);
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
        prepared_matchers: None,
        prepared_kinds: None,
        stored_indexes: None,
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
            dag.demand(node, &read, &mut context),
            Err(OperatingError::InvalidRegex { .. })
        ));
        assert!(dag.required[node.0].is_none());
        assert!(dag.ready[node.0].is_none());
        assert_eq!(dag.executions[node.0], 0);
        assert!(!dag.visiting[node.0]);
    }
    assert_eq!(reads.get(), 2, "failed results are never reused");
    dag.finish_rule(0);
    assert_eq!(dag.required_bytes, 0);
    assert_eq!(dag.retained_bytes, 0);
}
