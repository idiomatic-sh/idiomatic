//! The shared walk uses one join dependency contract for callback delivery and shares one source walk.
use super::super::super::fixtures::*;
use crate::boundary_fixture::fixtures::{FileRun, authored_request, corpus, ran, root, run_file};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId, Test};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::rows::types::{RowCount, Table};
#[test]
fn shared_walk_uses_the_same_join_dependency_contract_for_callback_delivery() {
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    let (engine, _) = engine();
    let mut authored = authored_request(2);
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: selection(Selector::Kind("absent-kind")),
                kept: vec![],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: selection(Selector::BadRegex),
                kept: vec![],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                }],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        },
    ];
    crate::boundary_fixture::fixtures::plans(&mut authored)[0]
        .emit
        .step = StepId(2);
    crate::boundary_fixture::fixtures::plans(&mut authored)[1].steps[0].operation =
        Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons: selection(Selector::Kind("other")),
            kept: vec![],
        };
    // The independent selector runs first, so the Join's failure, which stops the file, comes after it.
    crate::boundary_fixture::fixtures::plans(&mut authored).swap(0, 1);
    let program = engine.prepare(authored).unwrap();
    for diagnostic in [DiagnosticDelivery::Collected] {
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
        let (elements, mut symbols) = tree(FileId(0), b"abc");
        let intents = [
            FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            },
            FileRuleIntent::Run {
                diagnostic,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            },
        ];
        let mut rules = run(
            &request,
            (FileId(0), b"abc"),
            &elements,
            &intents,
            &mut symbols,
        );
        assert_eq!(
            complete(&mut rules, 0),
            expected(FileId(0), &elements, &[2])
        );
        // The LHS cannot match the file ("absent-kind"), so it publishes empty without a walk: the independent selector and the
        // Join's demanded RHS share the one head cohort, and a failed canonical Join commits no rows in either delivery.
        assert!(matches!(
            root(ran(&mut rules, 1)),
            Ok(RootOutcome::Failed(ref error)) if *error == regex_error("[")
        ));
        assert!(matches!(rules.end, FileEnd::Unfinished));
        request.finish();
    }
}
#[test]
fn independent_semantic_relation_selectors_share_one_source_walk() {
    use crate::boundary_fixture::fixtures::named_functions;
    use ryo_relations::domain::relations::{
        fixtures::node_id,
        types::{CellKind, Position, Value},
    };
    let (engine, semantics) = engine();
    let mut authored = authored_request(2);
    for (index, name) in ["main", "other"].iter().enumerate() {
        crate::boundary_fixture::fixtures::plans(&mut authored)[index].steps[0].operation =
            Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![compared(Test::Equal, 1, text(name))],
                kept: vec![ColumnIndex(0)],
            };
    }
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
    let table = named_functions(&mut symbols);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = TableFacts::new(&[(Relation::Function, &table)]);
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::DiagnosticAndObservations,
    }; 2];
    let mut rules = run_file(
        &request,
        FileRun {
            file: FileId(0),
            source: b"mainother",
            ..FileRun::of(&intents, &elements)
        },
        &mut symbols,
        &read,
    );
    for (rule, positions) in [(0, &[0_u32, 1][..]), (1, &[2_u32][..])] {
        assert_eq!(
            complete(&mut rules, rule),
            Table {
                kinds: vec![CellKind::Node],
                count: RowCount(positions.len()),
                cells: positions
                    .iter()
                    .map(|p| Value::Node(node_id(FileId(0), Position(*p))))
                    .collect(),
            },
        );
    }
    assert_eq!(
        semantics.selects(),
        0,
        "both selectors use shared row dispatch"
    );
    assert!(matches!(rules.end, FileEnd::Finished(_)));
    request.finish();
}
