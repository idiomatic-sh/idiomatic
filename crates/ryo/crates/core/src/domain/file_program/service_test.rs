//! Exact typed graph identity and an actual two-rule shared-cut execution.

use super::service::compile;
use super::types::FileProgram;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::service::pool_for;
use crate::domain::operator::types::PreparedPool;
use crate::domain::pipeline::service::{evaluate_file, evaluate_file_with_graph};
use crate::domain::pipeline::service_test::{KINDS, SOURCE, functions, plan};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::{ColumnsView, Relation};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOriginId, Input, LiteralText, Operand, ParameterId, QueryLiteral,
    RepeatedRows, StepId, Test,
};
use ryo_values::domain::plans::types::{Operation, Plan, PlanStep, StepScope};
use std::sync::Arc;

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
    let graph = Arc::new(compile(&plans));
    let shared = graph.cuts[0][0].expect("select before one local projection");
    assert_eq!(graph.cuts[1][0], Some(shared));
    let mut program = FileProgram::new(graph, |_| true, &[Relation::Function]);
    let mut symbols = Symbols::default();
    let columns = functions(&mut symbols);
    let rows = |relation| {
        (relation == Relation::Function).then_some(ColumnsView {
            kinds: KINDS,
            columns: &columns,
        })
    };
    let mut prepared = PreparedPool::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    for (rule, plan) in plans.iter().enumerate() {
        let mut context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(SOURCE),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        let mut graph_pool = pool_for(std::slice::from_ref(plan));
        evaluate_file_with_graph(
            plan,
            &rows,
            None,
            &mut context,
            &mut graph_pool,
            &mut prepared,
            &[],
            Some(&mut program),
            rule,
        )
        .unwrap();
        let mut ordinary_pool = pool_for(std::slice::from_ref(plan));
        let mut ordinary = PreparedPool::default();
        evaluate_file(
            plan,
            &rows,
            None,
            &mut context,
            &mut ordinary_pool,
            &mut ordinary,
            &[],
        )
        .unwrap();
        assert_eq!(graph_pool.steps[1], ordinary_pool.steps[1]);
        program.finish_rule(rule);
    }
    assert_eq!(program.shared_executions, 1, "second rule skips the prefix");
    assert!(program.peak_retained_cells > 0);
    assert!(program.peak_retained_cells <= 256 * 1024);
    assert!(program.entries.is_empty());
}

#[test]
fn failed_pure_cut_is_never_published_or_replayed() {
    let mut first = branched();
    if let Operation::Select { comparisons, .. } = &mut first.steps[0].operation {
        comparisons[0].test = Test::Matches;
        comparisons[0].right = Operand::Literal(QueryLiteral::Text(LiteralText("(".into())));
    }
    let plans = [first.clone(), first];
    let graph = Arc::new(compile(&plans));
    let mut program = FileProgram::new(graph, |_| true, &[Relation::Function]);
    let mut symbols = Symbols::default();
    let columns = functions(&mut symbols);
    let rows = |relation| {
        (relation == Relation::Function).then_some(ColumnsView {
            kinds: KINDS,
            columns: &columns,
        })
    };
    let mut prepared = PreparedPool::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    for (rule, plan) in plans.iter().enumerate() {
        let mut context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(SOURCE),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        let mut graph_pool = pool_for(std::slice::from_ref(plan));
        let failed = evaluate_file_with_graph(
            plan,
            &rows,
            None,
            &mut context,
            &mut graph_pool,
            &mut prepared,
            &[],
            Some(&mut program),
            rule,
        )
        .expect_err("invalid pattern");
        assert!(program.entries.is_empty(), "errors cannot enter the graph");
        let mut ordinary_pool = pool_for(std::slice::from_ref(plan));
        let mut ordinary = PreparedPool::default();
        let baseline = evaluate_file(
            plan,
            &rows,
            None,
            &mut context,
            &mut ordinary_pool,
            &mut ordinary,
            &[],
        )
        .expect_err("same invalid pattern");
        assert_eq!(failed, baseline);
        program.finish_rule(rule);
    }
    assert_eq!(program.shared_executions, 0);
}

#[test]
fn typed_graph_shares_only_complete_pure_cuts_and_releases_last_consumer() {
    let first = branched();
    let mut second = first.clone();
    if let Operation::Select { comparisons, .. } = &mut second.steps[0].operation {
        comparisons[0].origin = Some(ComparisonOriginId(7));
    }
    let plans = [first, second];
    let graph = Arc::new(compile(&plans));
    let shared = graph.cuts[0][0].expect("internal kept select");
    assert_eq!(
        graph.cuts[1][0],
        Some(shared),
        "source origins are caller owned"
    );
    let incomplete = FileProgram::new(Arc::clone(&graph), |_| true, &[]);
    assert_eq!(
        incomplete.remaining[shared.0], 0,
        "partial facts never mean complete empty"
    );
    assert!(incomplete.cut(0, 0).is_none());
    let mut dynamic = plans[0].clone();
    if let Operation::Select { comparisons, .. } = &mut dynamic.steps[0].operation {
        comparisons[0].right = Operand::Parameter(ParameterId(0));
    }
    assert!(
        compile(&[dynamic]).cuts[0][0].is_none(),
        "bound parameters refuse graph identity"
    );

    let mut symbols = Symbols::default();
    let columns = functions(&mut symbols);
    let rows = |relation| {
        (relation == Relation::Function).then_some(ColumnsView {
            kinds: KINDS,
            columns: &columns,
        })
    };
    let mut program = FileProgram::new(Arc::clone(&graph), |_| true, &[Relation::Function]);
    let mut prepared = PreparedPool::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut actual = Vec::new();
    for (rule, plan) in plans.iter().enumerate() {
        let mut pool = pool_for(std::slice::from_ref(plan));
        let mut context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(SOURCE),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        evaluate_file_with_graph(
            plan,
            &rows,
            None,
            &mut context,
            &mut pool,
            &mut prepared,
            &[],
            Some(&mut program),
            rule,
        )
        .unwrap();
        actual.push(pool.steps[2].clone());
        program.finish_rule(rule);
    }
    assert_eq!(
        program.shared_executions, 1,
        "one actual kept-cut execution was skipped"
    );
    assert!(program.peak_retained_cells > 0);
    assert!(program.peak_retained_cells <= 256 * 1024);
    assert!(
        program.entries.is_empty(),
        "last active consumer releases its table"
    );
    for (plan, result) in plans.iter().zip(actual) {
        let mut pool = pool_for(std::slice::from_ref(plan));
        let mut ordinary = PreparedPool::default();
        let mut context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(SOURCE),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        evaluate_file(
            plan,
            &rows,
            None,
            &mut context,
            &mut pool,
            &mut ordinary,
            &[],
        )
        .unwrap();
        assert_eq!(result, pool.steps[2]);
    }
}

#[test]
fn repeated_cuts_count_one_consumer_per_active_complete_rule() {
    let mut query = plan();
    query.steps.insert(1, query.steps[0].clone());
    query.emit.step = StepId(2);
    let graph = Arc::new(compile(&[query.clone(), query]));
    let node = graph.cuts[0][0].expect("first internal cut");
    assert_eq!(
        graph.cuts[0][1],
        Some(node),
        "duplicate occurrence stays addressable"
    );
    assert_eq!(graph.consumers, vec![vec![node], vec![node]]);
    let complete = FileProgram::new(Arc::clone(&graph), |_| true, &[Relation::Function]);
    assert_eq!(complete.remaining[node.0], 2, "two rules, not four cuts");
    let inactive = FileProgram::new(Arc::clone(&graph), |rule| rule == 1, &[Relation::Function]);
    assert_eq!(inactive.remaining[node.0], 1);
    let partial = FileProgram::new(graph, |_| true, &[]);
    assert_eq!(
        partial.remaining[node.0], 0,
        "partial populations have no consumers"
    );
}

#[test]
fn duplicate_cut_finishes_once_and_releases_only_after_last_rule() {
    use crate::domain::table_model::types::{RowCount, Table};
    use ryo_language::domain::relations::types::{CellKind, FileId, Value};
    let mut query = plan();
    query.steps.insert(1, query.steps[0].clone());
    query.emit.step = StepId(2);
    let graph = Arc::new(compile(&[query.clone(), query]));
    let node = graph.cuts[0][0].expect("internal cut");
    let mut program = FileProgram::new(graph, |_| true, &[Relation::Function]);
    let mut table = Table {
        kinds: vec![CellKind::File],
        cells: vec![Value::File(FileId(0))],
        count: RowCount(1),
    };
    drop(program.publish(node, &mut table).expect("two consumers"));
    program.finish_rule(0);
    assert_eq!(program.remaining[node.0], 1);
    assert_eq!(
        program.entries.len(),
        1,
        "second rule still owns a consumption"
    );
    program.finish_rule(1);
    assert_eq!(program.remaining[node.0], 0);
    assert!(
        program.entries.is_empty(),
        "last rule releases the retained generation"
    );
}

/// Exercise the actual executor after layout metadata has been prepared. A
/// resident-fact callback is an observable evaluator read, not a timing proxy.
fn demand_pruning_case(retained: bool, complete: bool, shared_branch: bool) -> (usize, u64) {
    use crate::domain::table_model::service::push_row;
    use crate::domain::table_model::types::Table;
    use ryo_language::domain::relations::types::{CellKind, FileId, Value};
    let mut query = plan();
    query.steps.push(PlanStep {
        operation: Operation::Project {
            from: Input::Step(StepId(1)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
        scope: StepScope::PerFile,
    });
    query.emit.step = StepId(2);
    if shared_branch {
        query.steps.push(PlanStep {
            operation: Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(1)],
                distinct: RepeatedRows::Kept,
            },
            scope: StepScope::PerFile,
        });
        query.frontier.push(Input::Step(StepId(3)));
    }
    let plans = [query.clone(), query];
    let mut program = FileProgram::new(
        Arc::new(compile(&plans)),
        |_| true,
        if complete { &[Relation::Function] } else { &[] },
    );
    if retained {
        let node = program.cut(1, 1).expect("complete shared project");
        let mut table = Table {
            kinds: vec![CellKind::File],
            ..Table::default()
        };
        push_row(&mut table, &[Value::File(FileId(10))]);
        push_row(&mut table, &[Value::File(FileId(11))]);
        drop(
            program
                .publish(node, &mut table)
                .expect("successful retained cut"),
        );
    }
    let mut symbols = Symbols::default();
    let columns = functions(&mut symbols);
    let reads = std::cell::Cell::new(0);
    let rows = |relation| {
        reads.set(reads.get() + 1);
        (relation == Relation::Function).then_some(ColumnsView {
            kinds: KINDS,
            columns: &columns,
        })
    };
    let mut prepared = PreparedPool::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let query = &plans[1];
    let mut ordinary = pool_for(std::slice::from_ref(query));
    evaluate_file(
        query,
        &rows,
        None,
        &mut context,
        &mut ordinary,
        &mut prepared,
        &[],
    )
    .unwrap();
    reads.set(0);
    let mut actual = pool_for(std::slice::from_ref(query));
    evaluate_file_with_graph(
        query,
        &rows,
        None,
        &mut context,
        &mut actual,
        &mut prepared,
        &[],
        Some(&mut program),
        1,
    )
    .unwrap();
    assert_eq!(actual.steps[2], ordinary.steps[2], "exact emitted rows");
    assert_eq!(actual.frontier, ordinary.frontier, "exact frontier rows");
    (reads.get(), program.shared_executions)
}

#[test]
fn cached_descendant_does_not_call_its_ancestor_evaluator() {
    let (reads, hits) = demand_pruning_case(true, true, false);
    assert_eq!(
        reads, 0,
        "cached descendant eliminates physical ancestor reads"
    );
    assert_eq!(hits, 1, "only the demanded retained cut counts as a hit");
}

#[test]
fn dependency_used_by_another_frontier_still_executes_and_cannot_evict_the_cut() {
    let (reads, hits) = demand_pruning_case(true, true, true);
    assert!(
        reads > 0,
        "the live branch still evaluates its shared ancestor"
    );
    assert_eq!(
        hits, 1,
        "an earlier publication cannot evict the pinned successful cut"
    );
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
