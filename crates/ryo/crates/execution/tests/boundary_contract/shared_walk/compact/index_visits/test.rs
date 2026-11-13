//! An indexed join answers the matching right-hand rows.
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
use ryo_relations::domain::relations::types::*;
use ryo_values::domain::plan_parts::types::*;
use ryo_values::domain::plans::types::*;
use ryo_values::domain::rows::types::{RowCount, Table};
#[test]
fn indexed_join_counts_actual_matching_rhs_visits_and_does_no_work_on_replay() {
    use ryo_values::domain::hosted::types::{
        HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
    };
    use ryo_values::domain::row_texts::types::Cell as HostCell;
    for (left, expected) in [
        (vec![7, 7, 9], vec![(7, 100), (7, 200), (7, 100), (7, 200)]),
        (vec![99, 99], vec![]),
    ] {
        let engine = engine();
        let mut authored = authored_request(1);
        authored.hosted = vec![
            HostRelation {
                name: HostName("left-keys".into()),
                columns: vec![HostColumn {
                    name: HostColumnName("key".into()),
                    kind: HostKind::Number,
                }],
                rows: left
                    .iter()
                    .map(|key| HostRow(vec![HostCell::Number(Integer(*key))]))
                    .collect(),
            },
            HostRelation {
                name: HostName("right-values".into()),
                columns: ["key", "value"]
                    .map(|name| HostColumn {
                        name: HostColumnName(name.into()),
                        kind: HostKind::Number,
                    })
                    .to_vec(),
                rows: [(7, 100), (7, 200), (8, 300), (10, 400), (11, 500)]
                    .into_iter()
                    .map(|(key, value)| {
                        HostRow(vec![
                            HostCell::Number(Integer(key)),
                            HostCell::Number(Integer(value)),
                        ])
                    })
                    .collect(),
            },
        ];
        let plan = &mut crate::boundary_fixture::fixtures::plans(&mut authored)[0];
        plan.steps = [0, 1]
            .map(|relation| PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Hosted {
                    relation: HostedNumber(relation),
                },
            })
            .to_vec();
        plan.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(0),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(1),
                    },
                ],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        });
        plan.emit.step = StepId(2);
        let program = engine.prepare(authored).unwrap();
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
        let mut symbols = Symbols::default();
        let elements = Elements::default();
        let read = TableFacts::new(&[]);
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
        let expected = Table {
            kinds: vec![CellKind::Number, CellKind::Number],
            count: RowCount(expected.len()),
            cells: expected
                .into_iter()
                .flat_map(|(key, value)| {
                    [Value::Number(Integer(key)), Value::Number(Integer(value))]
                })
                .collect(),
        };
        assert_eq!(complete(&mut rules, 0), expected);
        assert!(matches!(rules.end, FileEnd::Finished(_)));
        request.finish();
    }
}
