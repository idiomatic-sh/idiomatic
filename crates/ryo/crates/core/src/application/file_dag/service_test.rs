//! Count actual class-generic work and compare full ordered residual outputs.
use super::request::compile;
use super::types::FileDagExecution;
use crate::application::file_dag::pipeline::service::{evaluate_file, evaluate_file_with_dag};
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::{service::pool_for, types::PreparedPool};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::{kinds, to_bits},
    types::{ColumnsView, FactColumn, FileId, Integer, Relation, Value},
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input, JoinColumn, Operand, QueryLiteral, RepeatedRows, Side, StepId,
    Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};
use std::sync::Arc;

fn query(major: i64, join: bool) -> Plan {
    let mut steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: Input::Relation(Relation::LanguageTarget),
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Literal(QueryLiteral::Number(Integer(major))),
                origin: None,
            }],
            kept: vec![],
        },
    }];
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: if join {
            Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Relation(Relation::LanguageTarget),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                }],
                filter: vec![],
                view: vec![],
                reach: None,
            }
        } else {
            Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            }
        },
    });
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Project {
            from: Input::Step(StepId(1)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
    });
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps,
        emit: Emit {
            step: StepId(2),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    }
}
fn columns() -> Vec<FactColumn> {
    vec![
        FactColumn(
            [1, 2, 3]
                .map(|x| to_bits(Value::File(FileId(x))).0)
                .to_vec(),
        ),
        FactColumn(
            [3, 3, 2]
                .map(|x| to_bits(Value::Number(Integer(x))).0)
                .to_vec(),
        ),
        FactColumn(
            [11, 12, 9]
                .map(|x| to_bits(Value::Number(Integer(x))).0)
                .to_vec(),
        ),
    ]
}
fn context<'a>(
    symbols: &'a mut Symbols,
    memo: &'a mut Vec<crate::domain::operating::types::Memo>,
    scratch: &'a mut OperatorScratch,
) -> OperatorContext<'a> {
    OperatorContext {
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: symbols.into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo,
        scratch,
    }
}

#[test]
fn request_dag_executes_shared_dependencies_once_and_releases_at_graph_last_use() {
    let plans = [query(3, false), query(3, false)];
    let graph = Arc::new(compile(&plans));
    let root = graph.boundaries[0][0].1;
    let child = graph.cuts[0][0].unwrap();
    let mut dag = FileDagExecution::new(graph, |_| true, &[Relation::LanguageTarget]);
    let columns = columns();
    let rows = |relation| {
        (relation == Relation::LanguageTarget).then_some(ColumnsView {
            kinds: kinds(relation),
            columns: &columns,
        })
    };
    let mut symbols = Symbols::default();
    let mut memo = vec![];
    let mut scratch = OperatorScratch::default();
    let mut prepared = PreparedPool::default();
    dag.prepare_resources(
        true,
        &[Relation::LanguageTarget],
        &[],
        &[],
        &plans,
        &prepared,
        |_| true,
    );
    for (rule, plan) in plans.iter().enumerate() {
        let mut actual = pool_for(std::slice::from_ref(plan));
        let mut ordinary = pool_for(std::slice::from_ref(plan));
        let mut ordinary_prepared = PreparedPool::default();
        let mut ctx = context(&mut symbols, &mut memo, &mut scratch);
        evaluate_file(
            plan,
            &rows,
            None,
            &mut ctx,
            &mut ordinary,
            &mut ordinary_prepared,
            &[],
        )
        .unwrap();
        evaluate_file_with_dag(
            plan,
            &rows,
            None,
            &mut ctx,
            &mut actual,
            &mut prepared,
            &[],
            None,
            rule,
            Some(&std::cell::RefCell::new(&mut dag)),
        )
        .unwrap();
        assert_eq!(actual.steps[2], ordinary.steps[2]);
        assert_eq!(actual.steps[2].count.0, 2);
        assert!(
            dag.ready[child.0].is_none(),
            "producer releases after its unique graph consumer"
        );
        assert!(
            dag.ready[root.0].is_some(),
            "opaque rule consumers still need this output"
        );
        dag.finish_rule(rule);
    }
    assert_eq!(
        dag.executions.iter().sum::<u64>(),
        2,
        "one Select and one Project for both rules"
    );
    assert!(dag.executions.iter().all(|count| *count <= 1));
    assert_eq!(dag.hits, 1);
    assert!(dag.ready.iter().all(Option::is_none));
    assert_eq!(dag.retained_bytes, 0);
}

#[test]
fn distinct_graph_joins_share_the_existing_file_owned_source_index() {
    let plans = [query(3, true), query(2, true)];
    let mut dag = FileDagExecution::new(
        Arc::new(compile(&plans)),
        |_| true,
        &[Relation::LanguageTarget],
    );
    let columns = columns();
    let rows = |relation| {
        (relation == Relation::LanguageTarget).then_some(ColumnsView {
            kinds: kinds(relation),
            columns: &columns,
        })
    };
    let mut symbols = Symbols::default();
    let mut memo = vec![];
    let mut scratch = OperatorScratch::default();
    let mut prepared = PreparedPool::default();
    dag.prepare_resources(
        true,
        &[Relation::LanguageTarget],
        &[],
        &[],
        &plans,
        &prepared,
        |_| true,
    );
    for (rule, plan) in plans.iter().enumerate() {
        let mut actual = pool_for(std::slice::from_ref(plan));
        let mut ordinary = pool_for(std::slice::from_ref(plan));
        evaluate_file(
            plan,
            &rows,
            None,
            &mut context(&mut symbols, &mut memo, &mut scratch),
            &mut ordinary,
            &mut PreparedPool::default(),
            &[],
        )
        .unwrap();
        evaluate_file_with_dag(
            plan,
            &rows,
            None,
            &mut context(&mut symbols, &mut memo, &mut scratch),
            &mut actual,
            &mut prepared,
            &[],
            None,
            rule,
            Some(&std::cell::RefCell::new(&mut dag)),
        )
        .unwrap();
        assert_eq!(actual.steps[2], ordinary.steps[2]);
        dag.finish_rule(rule);
    }
    assert_eq!(
        dag.relations.derived.index_observation().builds,
        1,
        "same source/key has one actual index build"
    );
    assert_eq!(
        dag.relations.derived.index_observation().reuses,
        1,
        "second independent Join reuses the file handle"
    );
    assert_eq!(
        dag.executions.iter().sum::<u64>(),
        4,
        "two distinct Select/Join pairs"
    );
}

#[test]
fn incomplete_or_invalid_population_stays_entirely_on_the_residual_executor() {
    let plans = [query(3, false)];
    let graph = Arc::new(compile(&plans));
    let dag = FileDagExecution::new((graph).clone(), |_| true, &[]);
    assert!(!dag.active[0]);
    assert!(dag.owned(0, 0).is_none());
    let mut invalid = query(3, false);
    if let Operation::Project { columns, .. } = &mut invalid.steps[1].operation {
        columns[0] = ColumnIndex(999);
    }
    let dag = FileDagExecution::new(
        Arc::new(compile(&[invalid])),
        |_| true,
        &[Relation::LanguageTarget],
    );
    assert!(
        !dag.active[0],
        "typed failure cannot be reordered by graph admission"
    );
}

#[test]
fn opaque_first_failure_prevents_later_rule_graph_work() {
    let first = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::SourceMatches {
                from: Input::Relation(Relation::SourceLocation),
                span: ColumnIndex(2),
                base_line: None,
                pattern: ryo_values::domain::plan_parts::types::LiteralText("(".into()),
                capture: None,
                capture_text: false,
                capture_line: false,
            },
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(1),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    };
    let plans = [first, query(2, false)];
    let mut dag = FileDagExecution::new(
        Arc::new(compile(&plans)),
        |_| true,
        &[Relation::LanguageTarget],
    );
    let columns = columns();
    use ryo_relations::domain::relations::types::{NodeId, Offset, Span};
    let source = [
        Value::Node(NodeId(0)),
        Value::File(FileId(0)),
        Value::Span(Span {
            start: Offset(0),
            end: Offset(0),
        }),
        Value::Number(Integer(1)),
        Value::Number(Integer(1)),
        Value::Number(Integer(0)),
        Value::Number(Integer(0)),
        Value::Number(Integer(0)),
        Value::Number(Integer(0)),
    ]
    .map(|value| FactColumn(vec![to_bits(value).0]));
    let later_reads = std::cell::Cell::new(0);
    let rows = |relation| match relation {
        Relation::SourceLocation => Some(ColumnsView {
            kinds: kinds(relation),
            columns: &source,
        }),
        Relation::LanguageTarget => {
            later_reads.set(later_reads.get() + 1);
            Some(ColumnsView {
                kinds: kinds(relation),
                columns: &columns,
            })
        }
        _ => None,
    };
    let mut symbols = Symbols::default();
    let mut memo = vec![];
    let mut scratch = OperatorScratch::default();
    let mut pool = pool_for(&plans[..1]);
    let actual = evaluate_file_with_dag(
        &plans[0],
        &rows,
        None,
        &mut context(&mut symbols, &mut memo, &mut scratch),
        &mut pool,
        &mut PreparedPool::default(),
        &[],
        None,
        0,
        Some(&std::cell::RefCell::new(&mut dag)),
    )
    .unwrap_err();
    let ordinary = evaluate_file(
        &plans[0],
        &rows,
        None,
        &mut context(&mut symbols, &mut memo, &mut scratch),
        &mut pool,
        &mut PreparedPool::default(),
        &[],
    )
    .unwrap_err();
    assert_eq!(actual, ordinary);
    assert_eq!(
        later_reads.get(),
        0,
        "later rule facts are not acquired speculatively"
    );
    assert_eq!(
        dag.executions.iter().sum::<u64>(),
        0,
        "no speculation for later ordered terminal"
    );
}

#[test]
fn registered_union_preserves_bag_contract_and_shares_its_repeated_dependency() {
    let mut first = query(3, false);
    first.steps[1].operation = Operation::Union {
        left: Input::Step(StepId(0)),
        right: Input::Step(StepId(0)),
    };
    let plans = [first.clone(), first];
    let mut dag = FileDagExecution::new(
        Arc::new(compile(&plans)),
        |_| true,
        &[Relation::LanguageTarget],
    );
    let columns = columns();
    let rows = |relation| {
        (relation == Relation::LanguageTarget).then_some(ColumnsView {
            kinds: kinds(relation),
            columns: &columns,
        })
    };
    let mut symbols = Symbols::default();
    let mut memo = vec![];
    let mut scratch = OperatorScratch::default();
    for (rule, plan) in plans.iter().enumerate() {
        let mut actual = pool_for(std::slice::from_ref(plan));
        let mut ordinary = pool_for(std::slice::from_ref(plan));
        evaluate_file(
            plan,
            &rows,
            None,
            &mut context(&mut symbols, &mut memo, &mut scratch),
            &mut ordinary,
            &mut PreparedPool::default(),
            &[],
        )
        .unwrap();
        evaluate_file_with_dag(
            plan,
            &rows,
            None,
            &mut context(&mut symbols, &mut memo, &mut scratch),
            &mut actual,
            &mut PreparedPool::default(),
            &[],
            None,
            rule,
            Some(&std::cell::RefCell::new(&mut dag)),
        )
        .unwrap();
        assert_eq!(actual.steps[2], ordinary.steps[2]);
        dag.finish_rule(rule);
    }
    assert_eq!(dag.executions.iter().sum::<u64>(), 2);
    assert!(dag.executions.iter().all(|count| *count <= 1));
    assert_eq!(dag.retained_bytes, 0);
}

#[test]
fn budget_refusal_is_explicit_recomputation_and_never_stale_generation_reuse() {
    let plans = [query(3, false), query(3, false)];
    let graph = Arc::new(compile(&plans));
    let mut dag = FileDagExecution::new((graph).clone(), |_| true, &[Relation::LanguageTarget]);
    dag.limit = 0;
    let columns = columns();
    let rows = |relation| {
        (relation == Relation::LanguageTarget).then_some(ColumnsView {
            kinds: kinds(relation),
            columns: &columns,
        })
    };
    let mut symbols = Symbols::default();
    let mut memo = vec![];
    let mut scratch = OperatorScratch::default();
    for (rule, plan) in plans.iter().enumerate() {
        let mut pool = pool_for(std::slice::from_ref(plan));
        evaluate_file_with_dag(
            plan,
            &rows,
            None,
            &mut context(&mut symbols, &mut memo, &mut scratch),
            &mut pool,
            &mut PreparedPool::default(),
            &[],
            None,
            rule,
            Some(&std::cell::RefCell::new(&mut dag)),
        )
        .unwrap();
        assert_eq!(pool.steps[2].count.0, 2);
        dag.finish_rule(rule);
    }
    assert_eq!(dag.retained_bytes, 0);
    assert!(dag.unretained > 0);
    assert!(
        dag.executions.iter().any(|count| *count > 1),
        "recomputation is measured, never called once-only"
    );
    let next = FileDagExecution::new(graph, |_| true, &[Relation::LanguageTarget]);
    assert!(
        next.ready.iter().all(Option::is_none),
        "new file generation owns no old rows"
    );
}

#[test]
fn empty_left_join_never_executes_its_right_dependency_and_cancels_its_lifetime() {
    let mut plan = query(99, true);
    let right = query(2, false).steps[0].clone();
    plan.steps.insert(1, right);
    if let Operation::Join { right, .. } = &mut plan.steps[2].operation {
        *right = Input::Step(StepId(1));
    }
    if let Operation::Project { from, .. } = &mut plan.steps[3].operation {
        *from = Input::Step(StepId(2));
    }
    plan.emit.step = StepId(3);
    let graph = Arc::new(compile(std::slice::from_ref(&plan)));
    let unused = graph.cuts[0][1].unwrap();
    let mut dag = FileDagExecution::new(graph, |_| true, &[Relation::LanguageTarget]);
    let columns = columns();
    let rows = |relation| {
        (relation == Relation::LanguageTarget).then_some(ColumnsView {
            kinds: kinds(relation),
            columns: &columns,
        })
    };
    let mut symbols = Symbols::default();
    let mut memo = vec![];
    let mut scratch = OperatorScratch::default();
    let mut actual = pool_for(std::slice::from_ref(&plan));
    evaluate_file_with_dag(
        &plan,
        &rows,
        None,
        &mut context(&mut symbols, &mut memo, &mut scratch),
        &mut actual,
        &mut PreparedPool::default(),
        &[],
        None,
        0,
        Some(&std::cell::RefCell::new(&mut dag)),
    )
    .unwrap();
    assert_eq!(actual.steps[3].count.0, 0);
    assert_eq!(dag.executions[unused.0], 0, "lazy branch never runs");
    dag.finish_rule(0);
    assert!(dag.ready.iter().all(Option::is_none));
    assert!(dag.remaining.iter().all(|count| *count == 0));
    assert_eq!(dag.retained_bytes, 0);
}
