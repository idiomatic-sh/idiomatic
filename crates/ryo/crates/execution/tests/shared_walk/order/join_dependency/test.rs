//! The shared walk uses one join dependency contract for callback delivery and shares one source walk.
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use super::super::super::fixtures::*;
use super::super::super::no_catalog;
use crate::boundary_fixture::{authored_request, corpus, demand};
use ryo_execution::application::file_dag::program::request_execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::application::file_dag::program::file_session::interface::FileExecution;
use ryo_execution::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_relations::domain::relations::fixtures::kinds;
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
    crate::boundary_fixture::plans(&mut authored)[0].steps = vec![
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
    crate::boundary_fixture::plans(&mut authored)[0].emit.step = StepId(2);
    crate::boundary_fixture::plans(&mut authored)[1].steps[0].operation = Operation::Select {
        from: Input::Relation(Relation::SyntaxTree),
        comparisons: selection(Selector::Kind("other")),
        kept: vec![],
    };
    let program = engine.prepare(authored).unwrap();
    for diagnostic in [DiagnosticDelivery::Collected, DiagnosticDelivery::Forwarded] {
        for independent_first in [false, true] {
            let corpus = corpus();
            let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
            let (elements, mut symbols) = tree(FileId(0), b"abc");
            let intents = [
                FileRuleIntent::Run {
                    diagnostic,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                },
                FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                },
            ];
            let mut file = request
                .open_file(
                    input(FileId(0), b"abc", &elements, &intents, &no_catalog),
                    &mut symbols,
                )
                .unwrap();
            if independent_first {
                assert_eq!(
                    complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap()),
                    expected(FileId(0), &elements, &[2])
                );
            }
            for attempt in 0..2 {
                match diagnostic {
                    DiagnosticDelivery::Collected => {
                        let result = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
                        assert!(
                            matches!(result.emit, RootOutcome::Failed(ref error) if *error == regex_error("["))
                        );
                    }
                    DiagnosticDelivery::Forwarded => {
                        let mut delivered = Vec::new();
                        let result = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                            delivered.push(row.to_vec());
                            Ok(())
                        });
                        assert!(matches!(result, Err(ref error) if *error == regex_error("[")));
                        assert!(
                            delivered.is_empty(),
                            "a failed canonical Join commits no rows"
                        );
                    }
                    DiagnosticDelivery::Disabled => {
                        unreachable!("test declares a diagnostic consumer")
                    }
                }
                if attempt == 0 && !independent_first {
                    assert_eq!(
                        complete(file.rule(RuleIndex(1), demand(), &mut symbols).unwrap()),
                        expected(FileId(0), &elements, &[2])
                    );
                }
                // The independent selector and LHS reach the first actual head
                // cohort. Only LHS publication unlocks the Join's demanded RHS,
                // which reaches a second cohort. A terminal retry adds no work.
                counts(&file, 2, 8);
                assert_eq!(
                    file.walk_counts().predicate_rows.0,
                    2,
                    "the independent predicate and first failing RHS predicate each run once"
                );
            }
            file.finish().unwrap();
            request.finish();
        }
    }
}
#[test]
fn independent_semantic_relation_selectors_share_one_source_walk() {
    use crate::boundary_fixture::named_functions;
    use ryo_relations::domain::relations::{
        fixtures::node_id,
        types::{CellKind, Position, Value},
    };
    let (engine, semantics) = engine();
    let mut authored = authored_request(2);
    for (index, name) in ["main", "other"].iter().enumerate() {
        crate::boundary_fixture::plans(&mut authored)[index].steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::Function),
            comparisons: vec![compared(Test::Equal, 1, text(name))],
            kept: vec![ColumnIndex(0)],
        };
    }
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
    let table = named_functions(&mut symbols);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |relation| {
        assert_eq!(relation, Relation::Function);
        Ok(SourceRows::Computed(&table))
    };
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }; 2];
    let mut file = request
        .open_file(
            input(FileId(0), b"mainother", &elements, &intents, &read),
            &mut symbols,
        )
        .unwrap();
    for (rule, positions) in [(0, &[0_u32, 1][..]), (1, &[2_u32][..])] {
        assert_eq!(
            complete(file.rule(RuleIndex(rule), demand(), &mut symbols).unwrap()),
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
    counts(&file, 1, 3);
    assert_eq!(file.walk_counts().predicate_rows.0, 6);
    assert_eq!(
        semantics.selects(),
        0,
        "both selectors use shared row dispatch"
    );
    file.finish().unwrap();
    request.finish();
}
