//! Collected empty rank and span bounds keep the layout without demanding row errors.
use crate::mode_order::fixtures::*;
use crate::*;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::HostedNumber,
    row_texts::types::Cell as HostCell,
};

#[test]
fn collected_empty_rank_and_span_bounds_keep_layout_without_demanding_row_errors() {
    use ryo_values::domain::plan_parts::types::ParameterId;
    for rank in [false, true] {
        for populated in [false, true] {
            let mut input = authored_request(1);
            input.hosted = vec![HostRelation {
                name: HostName("text".into()),
                columns: vec![HostColumn {
                    name: HostColumnName("value".into()),
                    kind: HostKind::Text,
                }],
                rows: if populated {
                    vec![HostRow(vec![HostCell::Text("x".into())])]
                } else {
                    vec![]
                },
            }];
            boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
                step(Operation::Hosted {
                    relation: HostedNumber(0),
                }),
                step(if rank {
                    Operation::Rank {
                        from: Input::Step(StepId(0)),
                        column: ColumnIndex(0),
                        sets: ParameterId(99),
                    }
                } else {
                    Operation::SpanBounds {
                        from: Input::Step(StepId(0)),
                        span: ColumnIndex(99),
                    }
                }),
            ];
            boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(1);
            let outcome = evaluate(input, &TableFacts::new(&[]));
            if populated {
                if rank {
                    assert!(matches!(
                        outcome,
                        Ok(RootOutcome::Failed(OperatingError::Unbound))
                    ));
                } else {
                    assert_eq!(failed_column(outcome), Some(99));
                }
            } else {
                let Ok(RootOutcome::Complete(table)) = outcome else {
                    panic!("row errors stay unobserved on empty input")
                };
                assert_eq!(
                    table.kinds,
                    if rank {
                        vec![CellKind::Symbol, CellKind::Number]
                    } else {
                        vec![CellKind::Symbol, CellKind::Number, CellKind::Number]
                    }
                );
                assert_eq!(table.count, RowCount(0));
            }
        }
    }
}
