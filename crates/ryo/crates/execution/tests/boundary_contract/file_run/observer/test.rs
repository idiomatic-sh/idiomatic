//! A file run with and without the counting observer.
use super::super::super::*;

/// A file run answers the same rules whether or not an observer counts its work, and the counting observer is told the rows each
/// step of each rule was fed: the head fed every source row, its continuation only the rows the head kept.
#[test]
fn a_file_run_answers_the_same_with_or_without_the_counting_observer() {
    use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, Test};
    let engine = engine();
    let mut input = authored_request(1);
    let plan = &mut crate::boundary_fixture::fixtures::plans(&mut input)[0];
    plan.steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![PlanComparison {
                    test: Test::NotEqual,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Column(ColumnIndex(0)),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: vec![ColumnIndex(0)],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::GroupCount {
                from: Input::Step(StepId(0)),
                keys: vec![],
                distinct: None,
            },
        },
    ];
    plan.emit.step = StepId(1);
    let program = engine.prepare(input).expect("prepare a filtered count");
    let corpus = corpus();
    let facts = ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake::default();
    let run = |observer: Option<
        Arc<dyn ryo_execution::inbound::use_cases::execution::interface::ObserverPort>,
    >| {
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, observer.into());
        let mut worker = super::super::fixtures::function_rows(&facts, FileId(0), &[1, 2, 3]);
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut symbols = Symbols::default();
        let (ran, _) = crate::file_intents::fixtures::ran_file(
            &request,
            FileId(0),
            &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            }],
            (&elements, &mut symbols),
            worker.as_mut(),
        );
        request.finish();
        ran.into_iter()
            .map(|ran| (ran.complete, ran.after.starts_with("ran")))
            .collect::<Vec<_>>()
    };
    let unobserved = run(None);
    use ryo_execution::inbound::use_cases::execution::interface::ObserverPort as _;
    let counting = Arc::new(ryo_execution_runtime::outbound::observer::factory::counting());
    let shared = Arc::clone(&counting);
    let observer: Arc<dyn ryo_execution::inbound::use_cases::execution::interface::ObserverPort> =
        shared;
    let observed = run(Some(observer));
    assert_eq!(
        observed, unobserved,
        "the observer changes nothing the run answers"
    );
    assert!(matches!(observed.as_slice(), [(Some(_), true)]));
    let totals = counting
        .totals()
        .expect("the counting observer's totals")
        .0
        .iter()
        .map(|steps| steps.iter().map(|fed| fed.0).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(
        totals,
        [[[3, 0, 0], [0, 0, 0]]],
        "the head was fed the three source rows; the count, none, since the head kept none"
    );
}

/// S17: a rule's steps fused into one region are each counted for the rows they were fed: the head every row of the tree, its
/// fused continuation only the rows the head kept, as the counting observer is told through the one-call file route.
#[test]
fn each_fused_step_is_counted_for_the_rows_it_was_fed() {
    use crate::shared_walk::fixtures::{compared, text, tree};
    use ryo_execution::inbound::use_cases::execution::interface::ObserverPort as _;
    use ryo_values::domain::plan_parts::types::Test;
    let (engine, _) = crate::shared_walk::fixtures::engine();
    let mut input = authored_request(1);
    let plan = &mut crate::boundary_fixture::fixtures::plans(&mut input)[0];
    plan.steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: vec![compared(Test::Matches, 2, text("a"))],
                kept: vec![],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Step(StepId(0)),
                comparisons: vec![compared(Test::Equal, 1, text("word"))],
                kept: vec![],
            },
        },
    ];
    plan.emit.step = StepId(1);
    let program = engine
        .prepare(input)
        .expect("prepare a fused head and child");
    let corpus = corpus();
    let counting = Arc::new(ryo_execution_runtime::outbound::observer::factory::counting());
    let shared = Arc::clone(&counting);
    let observer: Arc<dyn ryo_execution::inbound::use_cases::execution::interface::ObserverPort> =
        shared;
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, Some(observer).into());
    let (elements, mut symbols) = tree(FileId(0), b"abc");
    let facts = ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake::default();
    let mut worker = facts.open_worker().expect("a fake worker");
    let (ran, _) = crate::file_intents::fixtures::ran_file(
        &request,
        FileId(0),
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }],
        (&elements, &mut symbols),
        worker.as_mut(),
    );
    request.finish();
    assert!(
        matches!(ran.as_slice(), [only] if only.after.starts_with("ran")),
        "the rule runs past its root"
    );
    let totals = counting
        .totals()
        .expect("the counting observer's totals")
        .0
        .iter()
        .map(|steps| steps.iter().map(|fed| fed.0).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(
        totals,
        [[[4, 0, 0], [1, 0, 0]]],
        "the head was fed the tree's four rows; its fused continuation, the one row the head kept"
    );
}
