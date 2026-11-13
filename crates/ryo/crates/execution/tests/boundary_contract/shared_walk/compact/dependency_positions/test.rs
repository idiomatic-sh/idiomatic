//! A grouped dependency and the original source join in both input positions.
use super::super::super::fixtures::complete;
use crate::boundary_fixture::fixtures::{FileRun, authored_request, corpus, engine, run_file};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::fixtures::kinds;
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
        crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps = vec![
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
        crate::boundary_fixture::fixtures::plans(&mut authored)[0]
            .emit
            .step = StepId(3);
        crate::boundary_fixture::fixtures::plans(&mut authored)[0]
            .emit
            .subject = ColumnIndex(0);
        let program = engine.prepare(authored).unwrap();
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
        let target = Table {
            kinds: kinds(Relation::LanguageTarget).to_vec(),
            count: RowCount(2),
            cells: vec![
                Value::File(FileId(0)),
                Value::Number(Integer(3)),
                Value::Number(Integer(7)),
                Value::File(FileId(0)),
                Value::Number(Integer(5)),
                Value::Number(Integer(9)),
            ],
        };
        let read = TableFacts::new(&[(Relation::LanguageTarget, &target)]);
        let elements = Elements::default();
        let mut symbols = Symbols::default();
        let intents = [FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }];
        let mut rules = run_file(
            &request,
            FileRun {
                file: FileId(0),
                source: b"",
                ..FileRun::of(&intents, &elements)
            },
            &mut symbols,
            &read,
        );
        assert_eq!(
            complete(&mut rules, 0),
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
        assert!(matches!(rules.end, FileEnd::Finished(_)));
        request.finish();
    }
}
