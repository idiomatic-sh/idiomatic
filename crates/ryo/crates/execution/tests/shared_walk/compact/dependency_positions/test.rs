//! A grouped dependency and the original source join in both input positions.
use ryo_execution::application::file_dag::program::file_session::interface::FileExecution as _;
use ryo_execution::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use super::super::super::fixtures::{complete, input};
use crate::boundary_fixture::{authored_request, corpus, demand, engine};
use ryo_execution::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::fixtures::{kinds, to_bits};
use ryo_relations::domain::relations::types::*;
use ryo_values::domain::plan_parts::types::*;
use ryo_values::domain::plans::types::*;
use ryo_values::domain::rows::types::{RowCount, Table};
#[test]
fn grouped_dependency_and_original_source_join_in_both_input_positions() {
    for reversed in [false, true] {
        let engine = engine();
        let mut authored = authored_request(1);
        let source = Input::Relation(Relation::LanguageTarget);
        crate::boundary_fixture::plans(&mut authored)[0].steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: source,
                    kept: vec![],
                    comparisons: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::GroupCount {
                    from: Input::Step(StepId(0)),
                    keys: vec![ColumnIndex(0)],
                    distinct: None,
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Step(StepId(1)),
                    kept: vec![],
                    comparisons: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Join {
                    left: if reversed {
                        source
                    } else {
                        Input::Step(StepId(2))
                    },
                    right: if reversed {
                        Input::Step(StepId(2))
                    } else {
                        source
                    },
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(0)],
                    output: vec![
                        JoinColumn {
                            side: if reversed { Side::Left } else { Side::Right },
                            column: ColumnIndex(1),
                        },
                        JoinColumn {
                            side: if reversed { Side::Right } else { Side::Left },
                            column: ColumnIndex(1),
                        },
                    ],
                    filter: vec![],
                    view: vec![],
                    reach: None,
                },
            },
        ];
        crate::boundary_fixture::plans(&mut authored)[0].emit.step = StepId(3);
        crate::boundary_fixture::plans(&mut authored)[0].emit.subject = ColumnIndex(0);
        let program = engine.prepare(authored).unwrap();
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
        let columns = [
            FactColumn(vec![to_bits(Value::File(FileId(0))).0; 2]),
            FactColumn(vec![
                to_bits(Value::Number(Integer(3))).0,
                to_bits(Value::Number(Integer(5))).0,
            ]),
            FactColumn(vec![
                to_bits(Value::Number(Integer(7))).0,
                to_bits(Value::Number(Integer(9))).0,
            ]),
        ];
        let read = |relation| {
            if relation == Relation::LanguageTarget {
                Ok(SourceRows::Stored(ColumnsView {
                    kinds: kinds(relation),
                    columns: &columns,
                }))
            } else {
                Err(OperatingError::NotRead)
            }
        };
        let elements = Elements::default();
        let mut symbols = Symbols::default();
        let intents = [FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }];
        let mut file = request
            .open_file(
                input(FileId(0), b"", &elements, &intents, &read),
                &mut symbols,
            )
            .unwrap();
        let outcome = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
        assert_eq!(
            complete(outcome),
            Table {
                kinds: vec![CellKind::Number, CellKind::Number],
                count: RowCount(2),
                cells: vec![
                    Value::Number(Integer(3)),
                    Value::Number(Integer(2)),
                    Value::Number(Integer(5)),
                    Value::Number(Integer(2))
                ],
            }
        );
        file.finish().unwrap();
        request.finish();
    }
}
