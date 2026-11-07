//! Hosted ordered dependencies use the same scheduler for both delivery modes.
use crate::boundary_fixture::{authored_request, corpus, demand, engine};
use ryo_execution::inbound::use_cases::execution::{interface::*, types::*};
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value};
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{ColumnIndex, HostedNumber, Input, JoinColumn, Side, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    row_texts::types::Cell,
    rows::types::{RowCount, Table},
};

fn ordered() -> PreparedRequest {
    let mut authored = authored_request(2);
    authored.hosted = [vec![1, 5], vec![2, 6]]
        .into_iter()
        .enumerate()
        .map(|(index, values)| HostRelation {
            name: HostName(format!("ordered-{index}").into()),
            columns: vec![HostColumn {
                name: HostColumnName("order".into()),
                kind: HostKind::Number,
            }],
            rows: values
                .into_iter()
                .map(|value| HostRow(vec![Cell::Number(Integer(value))]))
                .collect(),
        })
        .collect();
    for plan in &mut authored.plans {
        plan.steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Hosted {
                    relation: HostedNumber(0),
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Hosted {
                    relation: HostedNumber(1),
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::OrderedMatch {
                    left: Input::Step(StepId(0)),
                    right: Input::Step(StepId(1)),
                    left_on: vec![],
                    right_on: vec![],
                    left_order: ColumnIndex(0),
                    right_order: ColumnIndex(0),
                    output: vec![
                        JoinColumn {
                            side: Side::Left,
                            column: ColumnIndex(0),
                        },
                        JoinColumn {
                            side: Side::Right,
                            column: ColumnIndex(0),
                        },
                    ],
                },
            },
        ];
        plan.emit.step = StepId(2);
    }
    authored
}

#[test]
fn ordered_hosted_inputs_are_scheduled_once_before_collected_or_forwarded_delivery() {
    for diagnostic in [DiagnosticDelivery::Collected, DiagnosticDelivery::Forwarded] {
        let engine = engine();
        let program = engine.prepare(ordered()).unwrap();
        let corpus = corpus();
        let request = engine.begin(&program, &corpus);
        let elements = Elements::default();
        let read = |_| panic!("hosted inputs are immutable request data, never provider relations");
        let mut symbols = Symbols::default();
        let intents = [FileRuleIntent::Run {
            diagnostic,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }; 2];
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: facts::ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &intents,
                    source: b"abc",
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .unwrap();
        assert_eq!(file.readiness_counts().evaluations.0, 0);
        let mut first_work = None;
        for index in [1, 0, 1] {
            let actual = match diagnostic {
                DiagnosticDelivery::Collected => {
                    let RootOutcome::Complete(table) = file
                        .rule(RuleIndex(index), demand(), &mut symbols)
                        .unwrap()
                        .emit
                    else {
                        panic!("ordered collected result")
                    };
                    table
                }
                DiagnosticDelivery::Forwarded => {
                    let mut cells = vec![];
                    let mut rows = 0;
                    file.emit_rows(RuleIndex(index), &mut symbols, &mut |row| {
                        rows += 1;
                        cells.extend_from_slice(row);
                        Ok(())
                    })
                    .unwrap();
                    Table {
                        kinds: vec![CellKind::Number; 2],
                        count: RowCount(rows),
                        cells,
                    }
                }
                DiagnosticDelivery::Disabled => unreachable!(),
            };
            assert_eq!(
                actual,
                Table {
                    kinds: vec![CellKind::Number; 2],
                    count: RowCount(2),
                    cells: [1, 2, 5, 6]
                        .map(|value| Value::Number(Integer(value)))
                        .to_vec()
                }
            );
            let work = file.readiness_counts();
            assert!(
                work.evaluations.0 > 0,
                "ordered route must enter the canonical scheduler"
            );
            if let Some(first) = first_work {
                assert_eq!(
                    work, first,
                    "logical aliases cannot reschedule physical producers"
                );
            } else {
                first_work = Some(work);
            }
        }
        file.finish().unwrap();
        request.finish();
    }
}
