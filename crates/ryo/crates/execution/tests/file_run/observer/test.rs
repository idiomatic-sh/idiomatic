//! A file run with and without the counting observer.
use super::super::super::*;

/// A file run answers the same rules whether or not an observer counts its work, and the counting observer is told the rows each
/// step of each rule was fed: the head fed every source row, its continuation only the rows the head kept.
#[test]
fn a_file_run_answers_the_same_with_or_without_the_counting_observer() {
    use ryo_execution::inbound::use_cases::execution::interface::{
        CaptureContext, EmittedRoot, FileRunInput, RuleAfter,
    };
    use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, Test};
    let engine = engine();
    let mut input = authored_request(1);
    let plan = &mut crate::boundary_fixture::plans(&mut input)[0];
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
        Arc<dyn ryo_execution_runtime::outbound::observer::interface::ObserverPort>,
    >| {
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, observer);
        let worker = super::super::fixtures::function_rows(&facts, FileId(0), &[1, 2, 3]);
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut symbols = Symbols::default();
        let (mut arena, mut places, mut words) = (
            Vec::new(),
            Vec::new(),
            ryo_relations::domain::relations::types::Words::default(),
        );
        let rules = request.run_file(
            FileRunInput {
                language: SourceLanguage::Recognized,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::DiagnosticAndObservations,
                }],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
                first: &[ryo_values::domain::stored_facts::types::Bucket(0)],
            },
            &mut symbols,
            CaptureContext {
                worker: ryo_relations::domain::relations::types::WorkerNumber(0),
                arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                places: &mut places,
                words: &mut words,
            },
            worker.as_ref(),
        );
        request.finish();
        rules
            .rules
            .into_iter()
            .map(|ran| {
                let table = match ran.emit {
                    EmittedRoot::Delivered(RootOutcome::Complete(table)) => Some(table),
                    _ => None,
                };
                (table, matches!(ran.after, RuleAfter::Ran { .. }))
            })
            .collect::<Vec<_>>()
    };
    let unobserved = run(None);
    use ryo_execution_runtime::outbound::observer::interface::ObserverPort as _;
    let counting = Arc::new(ryo_execution_runtime::outbound::observer::factory::counting());
    let observed = run(Some(Arc::clone(&counting) as _));
    assert_eq!(observed, unobserved, "the observer changes nothing the run answers");
    assert!(matches!(observed.as_slice(), [(Some(_), true)]));
    let totals = counting
        .totals()
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

