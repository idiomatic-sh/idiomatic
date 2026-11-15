//! A collected empty rank keeps its layout without demanding its binding, which its first row checks; a span bounds whose span
//! column is past its input is refused before any row, empty or not, as its schema refuses it.
use crate::mode_order::fixtures::*;
use crate::*;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::HostedNumber,
    row_texts::types::Cell as HostCell,
};

#[test]
fn collected_empty_rank_keeps_layout_and_span_bounds_past_the_input_refuse_before_rows() {
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
            let outcome = evaluate(input, &mut TableFacts::new(&[]));
            match (rank, populated) {
                (true, true) => assert!(matches!(
                    outcome,
                    Ok(RootOutcome::Failed(OperatingError::Unbound))
                )),
                (true, false) => {
                    let Ok(RootOutcome::Complete(table)) = outcome else {
                        panic!("an unbound rank stays unobserved on empty input")
                    };
                    assert_eq!(table.kinds, vec![CellKind::Symbol, CellKind::Number]);
                    assert_eq!(table.count, RowCount(0));
                }
                // The span column is past the input: the schema refuses it, so the step is refused before any row.
                (false, _) => assert_eq!(failed_column(outcome), Some(99)),
            }
        }
    }
}
