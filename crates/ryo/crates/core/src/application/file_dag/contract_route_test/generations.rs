//! Actual DAG binding and file-generation isolation, independent of route counters.
use super::*;
fn binding_plan() -> Plan {
    let mut plan = plan(
        Operation::Select {
            from: Input::Relation(Relation::LanguageTarget),
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Parameter(ParameterId(0)),
                origin: None,
            }],
            kept: vec![],
        },
        StepScope::PerFile,
    );
    for from in [0, 1] {
        plan.steps.push(PlanStep {
            operation: Operation::Project {
                from: Input::Step(StepId(from)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
            scope: StepScope::PerFile,
        });
    }
    plan.emit.step = StepId(2);
    plan
}
fn language_columns(file: u32, major: i64) -> Vec<FactColumn> {
    vec![
        FactColumn(vec![to_bits(Value::File(FileId(file))).0]),
        FactColumn(vec![to_bits(Value::Number(Integer(major))).0]),
        FactColumn(vec![to_bits(Value::Number(Integer(10))).0]),
    ]
}
fn run_dag(
    plan: &Plan,
    operators: &[Arc<OperatorContract>],
    bindings: &[ParameterBinding],
    storage: &[FactColumn],
    rule: usize,
    dag: &mut FileDagExecution,
) -> Table {
    let read = |relation| {
        (relation == Relation::LanguageTarget).then_some(ColumnsView {
            kinds: kinds(relation),
            columns: storage,
        })
    };
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        prepared_matchers: None,
        prepared_kinds: None,
        bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut pool = pool_for(std::slice::from_ref(plan));
    evaluate_file_with_executor(
        plan,
        &read,
        None,
        &mut context,
        &mut pool,
        &mut PreparedPool::default(),
        &[],
        None,
        rule,
        Some(&RefCell::new(dag)),
        &CountingExecutor::default(),
        operators,
    )
    .unwrap();
    pool.steps.remove(2)
}
#[test]
fn identical_binding_dependent_operations_do_not_share_rule_graph_results() {
    let plans = [binding_plan(), binding_plan()];
    let bindings = vec![
        vec![ParameterBinding::Number(WholeNumber(3))],
        vec![ParameterBinding::Number(WholeNumber(2))],
    ];
    let operators = RequestOperators::prepare(&plans, &bindings, &[]).unwrap();
    let graph = Arc::new(compile_prepared(&plans, &operators));
    assert_ne!(graph.cuts[0][0], graph.cuts[1][0]);
    let mut dag = FileDagExecution::new(graph, |_| true, &[Relation::LanguageTarget]);
    let storage = language_columns(7, 3);
    let first = run_dag(
        &plans[0],
        &operators.rules[0],
        &bindings[0],
        &storage,
        0,
        &mut dag,
    );
    let second = run_dag(
        &plans[1],
        &operators.rules[1],
        &bindings[1],
        &storage,
        1,
        &mut dag,
    );
    assert_eq!(first.cells, vec![Value::File(FileId(7))]);
    assert_eq!(second.count.0, 0);
}
#[test]
fn two_file_generation_owners_cannot_reuse_the_other_files_results() {
    let plans = [binding_plan()];
    let bindings = vec![vec![ParameterBinding::Number(WholeNumber(3))]];
    let operators = RequestOperators::prepare(&plans, &bindings, &[]).unwrap();
    let graph = Arc::new(compile_prepared(&plans, &operators));
    let mut first = FileDagExecution::new((graph).clone(), |_| true, &[Relation::LanguageTarget]);
    let mut second = FileDagExecution::new(graph, |_| true, &[Relation::LanguageTarget]);
    let left = run_dag(
        &plans[0],
        &operators.rules[0],
        &bindings[0],
        &language_columns(1, 3),
        0,
        &mut first,
    );
    let right = run_dag(
        &plans[0],
        &operators.rules[0],
        &bindings[0],
        &language_columns(9, 3),
        0,
        &mut second,
    );
    assert_eq!(left.cells, vec![Value::File(FileId(1))]);
    assert_eq!(right.cells, vec![Value::File(FileId(9))]);
    assert!(first.executions.iter().sum::<u64>() > 0);
    assert!(second.executions.iter().sum::<u64>() > 0);
}
