//! Named function selection replays without running project steps or hiding absence.
use crate::boundary_fixture::fixtures::{FileRun, ran, root, run_file};
use crate::boundary_fixture::fixtures::{authored_request, corpus, engine};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::{
    EmittedRoot, FileEnd, FileStop, RuleAfter,
};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Relation, Value};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::RowCount,
};

/// The former per-file, missing-input and shared-pool scenarios use one prepared program.
#[test]
fn named_function_selection_replays_without_running_project_steps_or_hiding_absence() {
    use ryo_relations::domain::relations::{fixtures::node_id, types::Position};
    use ryo_values::domain::plan_parts::types::{
        LiteralText, Operand, PlanComparison, QueryLiteral, Test,
    };
    for absent in [false, true] {
        let engine = engine();
        let mut input = authored_request(2);
        let selected = PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: vec![
                    ColumnIndex(0),
                    ColumnIndex(1),
                    ColumnIndex(2),
                    ColumnIndex(3),
                ],
            },
        };
        let projected = PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
            },
        };
        crate::boundary_fixture::fixtures::plans(&mut input)[0].steps =
            vec![selected.clone(), projected.clone()];
        crate::boundary_fixture::fixtures::plans(&mut input)[0]
            .emit
            .step = StepId(1);
        crate::boundary_fixture::fixtures::plans(&mut input)[1].steps = vec![
            selected,
            projected,
            PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::GroupCount {
                    from: Input::Step(StepId(1)),
                    keys: vec![ColumnIndex(0)],
                    distinct: None,
                },
            },
        ];
        crate::boundary_fixture::fixtures::plans(&mut input)[1].frontier =
            vec![Input::Step(StepId(1))];
        crate::boundary_fixture::fixtures::plans(&mut input)[1]
            .emit
            .step = StepId(2);
        let program = engine.prepare(input).expect("compile both plans together");
        let mut corpus = corpus();
        if let ryo_values::domain::files::types::CorpusInput::Available(entry) = &mut corpus[0] {
            entry.size = ryo_values::domain::files::types::FileSize(9);
        }
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
        let mut symbols = Symbols::default();
        let table = crate::boundary_fixture::fixtures::named_functions(&mut symbols);
        let mut facts = if absent {
            TableFacts::new(&[])
        } else {
            TableFacts::new(&[(Relation::Function, &table)])
        };
        let elements = ryo_language::domain::elements::types::Elements::default();
        let collected = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        };
        let mut rules = run_file(
            &request,
            FileRun {
                source: b"mainother",
                ..FileRun::of(&[collected, collected], &elements)
            },
            &mut symbols,
            &mut facts,
        );
        let outcome = root(ran(&mut rules, 0));
        if absent {
            assert!(matches!(outcome, Err(FileStop::Facts(_))));
        } else {
            let Ok(RootOutcome::Complete(first)) = outcome else {
                panic!("matching declarations")
            };
            assert_eq!(first.count, RowCount(2));
            assert_eq!(
                first.cells,
                [
                    Value::Node(node_id(FileId(0), Position(0))),
                    Value::Node(node_id(FileId(0), Position(1)))
                ]
            );
            let project = ran(&mut rules, 1);
            assert!(matches!(
                project.emit,
                EmittedRoot::Delivered(RootOutcome::Deferred {
                    phase: StepScope::CrossFile
                })
            ));
            let RuleAfter::Ran { mut captures } = project.after else {
                panic!("project rule retains its file frontier")
            };
            assert_eq!(captures.len(), 1);
            let capture = captures.pop().unwrap().outcome.expect("complete frontier");
            let decoded = ryo_values::domain::table_words::fixtures::decode_complete(
                &capture.words,
                &mut vec![],
            )
            .unwrap();
            assert_eq!(
                decoded, first,
                "captured rows match the actual file selection"
            );
            assert!(matches!(rules.end, FileEnd::Finished(_)));
        }
        assert!(
            facts
                .history()
                .iter()
                .all(|relation| *relation == Relation::Function),
            "the rules read Function alone"
        );
        request.finish();
    }
}
