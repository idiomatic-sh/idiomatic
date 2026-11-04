//! Request-graph sharing, exact rule roots, and last-consumer release under the sole scheduler.

use crate::application::file_dag::{
    interface::DagExecution as _,
    pipeline::{
        service::evaluate_file_with_executor,
        service_test::{KINDS, SOURCE, functions, plan},
    },
    testing::FileScope,
};
use crate::domain::{
    file_program::service::compile_prepared,
    operating::{errors::OperatingError, types::OperatorContext},
    operator::{service::pool_for, types::PreparedPool},
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, FileId, Relation, Value,
};
use ryo_values::domain::{
    plan_parts::types::{
        ColumnIndex, ComparisonOriginId, Input, LiteralText, Operand, ParameterId, QueryLiteral,
        RepeatedRows, StepId, Test,
    },
    plans::types::{Operation, Plan, PlanStep, StepScope},
    rows::types::{RowCount, Table},
};
use std::{cell::Cell, sync::Arc};

fn request(
    plans: &[Plan],
    bindings: &[Vec<crate::domain::table_model::types::ParameterBinding>],
) -> crate::domain::operator::contract::types::RequestOperators {
    crate::domain::operator::contract::types::RequestOperators::prepare_with_schemas(
        plans,
        bindings,
        &[],
        &|relation| {
            if relation == Relation::Function {
                KINDS
            } else {
                ryo_relations::domain::relations::service::kinds(relation)
            }
        },
        crate::application::file_dag::execution::CommonOperatorEngine::fixture_ref()
            .derived
            .clone(),
    )
    .expect("fixture request schema")
}

fn graph(plans: &[Plan]) -> Arc<crate::domain::file_program::types::RequestGraph> {
    Arc::new(compile_prepared(
        plans,
        &request(plans, &vec![Vec::new(); plans.len()]),
    ))
}

fn scope(plans: &[Plan], complete: &[Relation]) -> FileScope<'static> {
    let graph = graph(plans);
    let fixture = FileScope::new(
        graph,
        crate::application::file_dag::execution::CommonOperatorEngine::fixture_ref(),
        |_| true,
        complete,
        &[],
    )
    .expect("file scope");
    fixture.execution.borrow().prepare_resources(
        true,
        complete,
        &[],
        complete,
        plans,
        &PreparedPool::default(),
        |_| true,
    );
    fixture
}

fn execute<'f>(
    fixture: &FileScope<'_>,
    plan: &Plan,
    rule: usize,
    read: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    symbols: &mut Symbols,
) -> Result<Table, OperatingError> {
    execute_results(fixture, plan, rule, read, symbols).map(|(emitted, _)| emitted)
}

fn execute_results<'f>(
    fixture: &FileScope<'_>,
    plan: &Plan,
    rule: usize,
    read: &dyn Fn(Relation) -> Option<ColumnsView<'f>>,
    symbols: &mut Symbols,
) -> Result<(Table, Vec<Table>), OperatingError> {
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: symbols.into(),
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut pool = pool_for(std::slice::from_ref(plan));
    let mut prepared = PreparedPool::default();
    let result = evaluate_file_with_executor(
        plan,
        read,
        None,
        &mut context,
        &mut pool,
        &mut prepared,
        rule,
        &fixture.execution,
        fixture.scope.executor(),
    );
    fixture.execution.borrow_mut().finish_rule(rule);
    result?;
    let emitted = pool
        .steps
        .get(plan.emit.step.0 as usize)
        .cloned()
        .ok_or(OperatingError::NotRun)?;
    Ok((emitted, pool.frontier))
}

fn branched() -> Plan {
    let mut plan = plan();
    plan.steps.push(PlanStep {
        operation: Operation::Project {
            from: Input::Step(StepId(0)),
            columns: vec![ColumnIndex(1)],
            distinct: RepeatedRows::Kept,
        },
        scope: StepScope::PerFile,
    });
    plan.emit.step = StepId(2);
    plan
}

#[test]
fn global_dag_keeps_a_prefix_with_one_reader_per_rule() {
    let plans = [plan(), plan()];
    let fixture = scope(&plans, &[Relation::Function]);
    let shared = fixture.scope.graph().cuts[0][0].expect("shared select");
    assert_eq!(fixture.scope.graph().cuts[1][0], Some(shared));
    let mut symbols = Symbols::default();
    let columns = functions(&mut symbols);
    let read = |relation| {
        (relation == Relation::Function).then_some(ColumnsView {
            kinds: KINDS,
            columns: &columns,
        })
    };
    let first = execute(&fixture, &plans[0], 0, &read, &mut symbols).unwrap();
    let second = execute(&fixture, &plans[1], 1, &read, &mut symbols).unwrap();
    assert_eq!(first, second);
    let state = fixture.execution.borrow();
    assert_eq!(state.executions[shared.0], 1, "physical select is shared");
    assert!(state.hits > 0, "second rule reuses the publication");
    assert!(state.peak_bytes <= state.limit);
    assert!(
        state.ready[shared.0].is_none(),
        "last rule releases the cut"
    );
}

#[test]
fn failed_pure_cut_is_never_published_or_replayed() {
    let mut first = branched();
    if let Operation::Select { comparisons, .. } = &mut first.steps[0].operation {
        comparisons[0].test = Test::Matches;
        comparisons[0].right = Operand::Literal(QueryLiteral::Text(LiteralText("(".into())));
    }
    let plans = [first.clone(), first];
    let fixture = scope(&plans, &[Relation::Function]);
    let shared = fixture.scope.graph().cuts[0][0].expect("shared failing select");
    let mut symbols = Symbols::default();
    let columns = functions(&mut symbols);
    let read = |relation| {
        (relation == Relation::Function).then_some(ColumnsView {
            kinds: KINDS,
            columns: &columns,
        })
    };
    let first = execute(&fixture, &plans[0], 0, &read, &mut symbols).unwrap_err();
    let second = execute(&fixture, &plans[1], 1, &read, &mut symbols).unwrap_err();
    assert_eq!(first, second, "both roots retain the original refusal");
    let state = fixture.execution.borrow();
    assert!(
        state.ready[shared.0].is_none(),
        "failed output never enters cache"
    );
    assert!(
        state.failed[shared.0].is_some(),
        "failure is retained for the next root"
    );
}

#[test]
fn typed_graph_shares_only_complete_pure_cuts_and_releases_last_consumer() {
    let first = branched();
    let mut second = first.clone();
    if let Operation::Select { comparisons, .. } = &mut second.steps[0].operation {
        comparisons[0].origin = Some(ComparisonOriginId(7));
    }
    let plans = [first, second];
    let graph = graph(&plans);
    let shared = graph.cuts[0][0].expect("shared pure select");
    assert_eq!(
        graph.cuts[1][0],
        Some(shared),
        "diagnostic origin is caller-owned"
    );
    let incomplete = crate::application::file_dag::activation::types::ActivationCache::default()
        .activate(&graph, |_| true, &[]);
    assert!(
        !incomplete.reusable[shared.0],
        "partial source cannot certify reusable result"
    );

    let mut dynamic = plans[0].clone();
    if let Operation::Select { comparisons, .. } = &mut dynamic.steps[0].operation {
        comparisons[0].right = Operand::Parameter(ParameterId(0));
    }
    let bound_plans = [dynamic.clone(), dynamic];
    let bound_values = [
        vec![crate::domain::table_model::types::ParameterBinding::Text(
            LiteralText("main".into()),
        )],
        vec![crate::domain::table_model::types::ParameterBinding::Text(
            LiteralText("other".into()),
        )],
    ];
    let bound = Arc::new(compile_prepared(
        &bound_plans,
        &request(&bound_plans, &bound_values),
    ));
    assert_ne!(
        bound.program_nodes[0][0].as_ref().unwrap(),
        bound.program_nodes[1][0].as_ref().unwrap(),
        "different parameter bindings cannot share a physical node"
    );
    assert_ne!(
        bound.cuts[0][0], bound.cuts[1][0],
        "parameterized cuts retain separate physical identities"
    );

    let fixture = scope(&plans, &[Relation::Function]);
    let mut symbols = Symbols::default();
    let columns = functions(&mut symbols);
    let read = |relation| {
        (relation == Relation::Function).then_some(ColumnsView {
            kinds: KINDS,
            columns: &columns,
        })
    };
    let first = execute(&fixture, &plans[0], 0, &read, &mut symbols).unwrap();
    let second = execute(&fixture, &plans[1], 1, &read, &mut symbols).unwrap();
    assert_eq!(first, second);
    let state = fixture.execution.borrow();
    assert_eq!(state.executions[shared.0], 1);
    assert!(
        state.ready[shared.0].is_none(),
        "last consumer releases shared storage"
    );
}

#[test]
fn repeated_cuts_count_one_consumer_per_active_complete_rule() {
    let mut query = plan();
    query.steps.insert(1, query.steps[0].clone());
    query.emit.step = StepId(2);
    query.frontier.push(Input::Step(StepId(1)));
    let duplicated = graph(&[query.clone(), query]);
    let node = duplicated.cuts[0][0].expect("first internal cut");
    assert_eq!(
        duplicated.program_nodes[0][1].as_ref().ok().copied(),
        Some(node),
        "the exported duplicate is a distinct boundary for the same physical node"
    );
    let complete = crate::application::file_dag::activation::types::ActivationCache::default()
        .activate(&duplicated, |_| true, &[Relation::Function]);
    let inactive = crate::application::file_dag::activation::types::ActivationCache::default()
        .activate(&duplicated, |rule| rule == 1, &[Relation::Function]);
    let partial = crate::application::file_dag::activation::types::ActivationCache::default()
        .activate(&duplicated, |_| true, &[]);
    assert!(complete.remaining[node.0] > inactive.remaining[node.0]);
    let mut deduplicated = plan();
    deduplicated.frontier.push(Input::Step(StepId(0)));
    let equivalent = graph(&[deduplicated.clone(), deduplicated]);
    let equivalent_node = *equivalent.program_nodes[0][0]
        .as_ref()
        .expect("equivalent source node");
    let equivalent_schedule =
        crate::application::file_dag::activation::types::ActivationCache::default().activate(
            &equivalent,
            |_| true,
            &[Relation::Function],
        );
    assert_eq!(
        complete.remaining[node.0], equivalent_schedule.remaining[equivalent_node.0],
        "duplicate source spellings do not inflate physical reader counts"
    );
    assert!(complete.reusable[node.0]);
    assert!(
        !partial.reusable[node.0],
        "partiality changes reuse, not dependency scheduling"
    );
    assert_eq!(partial.remaining[node.0], complete.remaining[node.0]);
}

#[test]
fn provenance_admission_keeps_unrelated_internal_streams_lazy() {
    let mut query = plan();
    let mut other = query.steps[0].clone();
    if let Operation::Select { comparisons, .. } = &mut other.operation {
        comparisons[0].right = Operand::Literal(QueryLiteral::Text(LiteralText("other".into())));
    }
    query.steps[1] = other;
    query.steps.push(PlanStep {
        operation: Operation::Union {
            left: Input::Step(StepId(0)),
            right: Input::Step(StepId(1)),
        },
        scope: StepScope::PerFile,
    });
    query.emit.step = StepId(2);
    let graph = graph(&[query]);
    let union = *graph.program_nodes[0][2].as_ref().expect("compiled union");
    assert_eq!(
        graph.nodes[union.0].input_supply,
        vec![
            crate::domain::operating::types::Supply::Streamed,
            crate::domain::operating::types::Supply::Streamed,
        ],
        "only inputs named by a prepared derived use gain held provenance"
    );
}

#[test]
fn ordinary_join_and_group_count_keep_single_reader_streams() {
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    let mut query = plan();
    query.steps.truncate(1);
    query.steps.push(PlanStep {
        operation: Operation::Join {
            left: Input::Step(StepId(0)),
            right: Input::Relation(Relation::Function),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![JoinColumn {
                side: Side::Left,
                column: ColumnIndex(1),
            }],
            filter: vec![],
            view: vec![],
            reach: None,
        },
        scope: StepScope::PerFile,
    });
    query.emit.step = StepId(1);
    let joined = graph(&[query]);
    let id = *joined.program_nodes[0][1].as_ref().expect("compiled join");
    assert_eq!(
        joined.nodes[id.0].input_supply[0],
        crate::domain::operating::types::Supply::Streamed,
    );
    assert!(!joined.nodes[id.0].native_join_right_first);

    let mut query = plan();
    query.steps.truncate(1);
    query.steps.push(PlanStep {
        operation: Operation::GroupCount {
            from: Input::Step(StepId(0)),
            keys: vec![ColumnIndex(0)],
            distinct: None,
        },
        scope: StepScope::PerFile,
    });
    query.emit.step = StepId(1);
    let grouped = graph(&[query]);
    let id = *grouped.program_nodes[0][1]
        .as_ref()
        .expect("compiled GroupCount");
    assert_eq!(
        grouped.nodes[id.0].input_supply[0],
        crate::domain::operating::types::Supply::Streamed,
    );
    assert!(!grouped.nodes[id.0].native_join_right_first);
}

#[test]
fn duplicate_cut_finishes_once_and_releases_only_after_last_rule() {
    let mut query = plan();
    query.steps.insert(1, query.steps[0].clone());
    query.emit.step = StepId(2);
    query.frontier.push(Input::Step(StepId(1)));
    let plans = [query.clone(), query];
    let fixture = scope(&plans, &[Relation::Function]);
    let node = fixture.scope.graph().cuts[0][0].expect("shared cut");
    let mut symbols = Symbols::default();
    let columns = functions(&mut symbols);
    let read = |relation| {
        (relation == Relation::Function).then_some(ColumnsView {
            kinds: KINDS,
            columns: &columns,
        })
    };
    let first = execute(&fixture, &plans[0], 0, &read, &mut symbols).unwrap();
    assert!(fixture.execution.borrow().remaining[node.0] > 0);
    assert!(fixture.execution.borrow().ready[node.0].is_some());
    let second = execute(&fixture, &plans[1], 1, &read, &mut symbols).unwrap();
    assert_eq!(first, second);
    let state = fixture.execution.borrow();
    assert_eq!(state.executions[node.0], 1);
    assert_eq!(state.remaining[node.0], 0);
    assert!(
        state.ready[node.0].is_none(),
        "last rule releases retained generation"
    );
}

/// Resident-fact callbacks observe actual physical reads, not elapsed time.
fn demand_pruning_case(retained: bool, complete: bool, shared_branch: bool) -> (usize, u64) {
    let first = plan();
    let mut second = first.clone();
    if shared_branch {
        second.steps.push(PlanStep {
            operation: Operation::Project {
                from: Input::Relation(Relation::Function),
                columns: vec![ColumnIndex(1)],
                distinct: RepeatedRows::Kept,
            },
            scope: StepScope::PerFile,
        });
        second.frontier.push(Input::Step(StepId(2)));
    }
    let plans = [first, second];
    let complete_relations: &[Relation] = if complete { &[Relation::Function] } else { &[] };
    let fixture = scope(&plans, complete_relations);
    let mut symbols = Symbols::default();
    let columns: Vec<FactColumn> = functions(&mut symbols);
    let reads = Cell::new(0);
    let read = |relation| {
        reads.set(reads.get() + 1);
        (relation == Relation::Function).then_some(ColumnsView {
            kinds: KINDS,
            columns: &columns,
        })
    };
    let first_result =
        retained.then(|| execute(&fixture, &plans[0], 0, &read, &mut symbols).unwrap());
    reads.set(0);
    let before_hits = fixture.execution.borrow().hits;
    let (second, frontier) = execute_results(&fixture, &plans[1], 1, &read, &mut symbols).unwrap();
    if shared_branch {
        assert_eq!(
            frontier,
            vec![Table {
                kinds: vec![CellKind::File],
                cells: vec![
                    Value::File(FileId(10)),
                    Value::File(FileId(11)),
                    Value::File(FileId(11)),
                ],
                count: RowCount(3),
            }],
            "a live frontier keeps its exact rows while the emitted cut is reused"
        );
    } else {
        assert!(frontier.is_empty());
    }
    if let Some(first) = first_result {
        assert_eq!(first, second, "shared emitted rows stay exact");
    }
    let hits = fixture.execution.borrow().hits - before_hits;
    (reads.get(), hits)
}

#[test]
fn cached_descendant_does_not_call_its_ancestor_evaluator() {
    let (reads, hits) = demand_pruning_case(true, true, false);
    assert_eq!(reads, 0, "retained result avoids the physical source read");
    assert!(hits > 0);
}

#[test]
fn dependency_used_by_another_frontier_still_executes_and_cannot_evict_the_cut() {
    let (reads, hits) = demand_pruning_case(true, true, true);
    assert!(
        reads > 0,
        "independent live frontier still reads its source"
    );
    assert!(hits > 0, "the prior successful cut remains reusable");
}

#[test]
fn cache_miss_runs_ancestors() {
    let (reads, hits) = demand_pruning_case(false, true, false);
    assert!(reads > 0);
    assert_eq!(hits, 0);
}

#[test]
fn partial_population_never_cuts_ancestor_execution() {
    let (reads, hits) = demand_pruning_case(false, false, false);
    assert!(reads > 0);
    assert_eq!(hits, 0);
}
