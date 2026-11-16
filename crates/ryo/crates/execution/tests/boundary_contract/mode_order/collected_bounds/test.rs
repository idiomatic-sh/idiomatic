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
            let outcome = evaluate(input, &mut TableFacts::new(&[]));
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

/// What the interpreter at checkpoint-350 (64a99968a1) answered for each row-by-row step, over an empty and a one-row hosted
/// `text(value: Text, n: Number)`, emitted itself or read by a projection of its column 0: run on that engine, recorded.
const ROW_BY_ROW_REFUSALS: &str = concat!(
    "capture-col99 populated=false downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "capture-col99 populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "capture-col99 populated=true downstream=false: failed Table(NoSuchColumn { index: 99 })\n",
    "capture-col99 populated=true downstream=true: failed Table(NoSuchColumn { index: 99 })\n",
    "capture-num populated=false downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "capture-num populated=false downstream=true: complete [Symbol] RowCount(0)\n",
    "capture-num populated=true downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "capture-num populated=true downstream=true: complete [Symbol] RowCount(0)\n",
    "rank-col99 populated=false downstream=false: complete [Symbol, Number, Number] RowCount(0)\n",
    "rank-col99 populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "rank-col99 populated=true downstream=false: failed Unbound\n",
    "rank-col99 populated=true downstream=true: failed Unbound\n",
    "words-col99 populated=false downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "words-col99 populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "words-col99 populated=true downstream=false: failed Table(NoSuchColumn { index: 99 })\n",
    "words-col99 populated=true downstream=true: failed Table(NoSuchColumn { index: 99 })\n",
    "words-num populated=false downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "words-num populated=false downstream=true: complete [Symbol] RowCount(0)\n",
    "words-num populated=true downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "words-num populated=true downstream=true: complete [Symbol] RowCount(0)\n",
    "replace-col99 populated=false downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "replace-col99 populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "replace-col99 populated=true downstream=false: failed Table(NoSuchColumn { index: 99 })\n",
    "replace-col99 populated=true downstream=true: failed Table(NoSuchColumn { index: 99 })\n",
    "replace-num populated=false downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "replace-num populated=false downstream=true: complete [Symbol] RowCount(0)\n",
    "replace-num populated=true downstream=false: failed ShapeMismatch\n",
    "replace-num populated=true downstream=true: failed ShapeMismatch\n",
    "replace-badre populated=false downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "replace-badre populated=false downstream=true: complete [Symbol] RowCount(0)\n",
    "replace-badre populated=true downstream=false: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }\n",
    "replace-badre populated=true downstream=true: failed InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }\n",
    "spanbetween-col99 populated=false downstream=false: complete [Symbol, Number, Span] RowCount(0)\n",
    "spanbetween-col99 populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbetween-col99 populated=true downstream=false: failed Table(NoSuchColumn { index: 99 })\n",
    "spanbetween-col99 populated=true downstream=true: failed Table(NoSuchColumn { index: 99 })\n",
    "spanbetween-text populated=false downstream=false: complete [Symbol, Number, Span] RowCount(0)\n",
    "spanbetween-text populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbetween-text populated=true downstream=false: failed ShapeMismatch\n",
    "spanbetween-text populated=true downstream=true: failed ShapeMismatch\n",
    "sourcematches-col99 populated=false downstream=false: complete [Symbol, Number, Span] RowCount(0)\n",
    "sourcematches-col99 populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "sourcematches-col99 populated=true downstream=false: failed Table(NoSuchColumn { index: 99 })\n",
    "sourcematches-col99 populated=true downstream=true: failed Table(NoSuchColumn { index: 99 })\n",
    "sourcematches-text populated=false downstream=false: complete [Symbol, Number, Span] RowCount(0)\n",
    "sourcematches-text populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "sourcematches-text populated=true downstream=false: failed ShapeMismatch\n",
    "sourcematches-text populated=true downstream=true: failed ShapeMismatch\n",
    "render-col99 populated=false downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "render-col99 populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "render-col99 populated=true downstream=false: failed Table(NoSuchColumn { index: 99 })\n",
    "render-col99 populated=true downstream=true: failed Table(NoSuchColumn { index: 99 })\n",
    "render-text populated=false downstream=false: complete [Symbol, Number, Symbol] RowCount(0)\n",
    "render-text populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "render-text populated=true downstream=false: failed ShapeMismatch\n",
    "render-text populated=true downstream=true: failed ShapeMismatch\n",
    "fits-col99 populated=false downstream=false: complete [Symbol, Number] RowCount(0)\n",
    "fits-col99 populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "fits-col99 populated=true downstream=false: failed ShapeMismatch\n",
    "fits-col99 populated=true downstream=true: failed ShapeMismatch\n",
    "fits-spantext populated=false downstream=false: complete [Symbol, Number] RowCount(0)\n",
    "fits-spantext populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "fits-spantext populated=true downstream=false: failed ShapeMismatch\n",
    "fits-spantext populated=true downstream=true: failed ShapeMismatch\n",
    "spanbounds-col99 populated=false downstream=false: complete [Symbol, Number, Number, Number] RowCount(0)\n",
    "spanbounds-col99 populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbounds-col99 populated=true downstream=false: failed Table(NoSuchColumn { index: 99 })\n",
    "spanbounds-col99 populated=true downstream=true: failed Table(NoSuchColumn { index: 99 })\n",
    "spanbounds-text populated=false downstream=false: complete [Symbol, Number, Number, Number] RowCount(0)\n",
    "spanbounds-text populated=false downstream=true: failed Table(NoSuchColumn { index: 0 })\n",
    "spanbounds-text populated=true downstream=false: failed ShapeMismatch\n",
    "spanbounds-text populated=true downstream=true: failed ShapeMismatch\n",
);

/// A row-by-row operator refuses its operand at the first row that reaches it: over no rows it answers the layout its rows
/// keep. A step that reads it reads no column of it, so a projection of its column 0 is refused once the step ran without a
/// refusal of its own, and with the step's own refusal first when a row reached it.
#[test]
fn row_by_row_refusals_and_their_readers_answer_as_the_interpreter() {
    let mut answered = String::new();
    for (name, operation) in row_by_row_steps() {
        for populated in [false, true] {
            for downstream in [false, true] {
                let reader = downstream.then(|| project(&[0]));
                let outcome = row_by_row(operation.clone(), populated, reader);
                answered.push_str(&format!(
                    "{name} populated={populated} downstream={downstream}: {outcome}\n"
                ));
            }
        }
    }
    assert_eq!(answered, ROW_BY_ROW_REFUSALS);
}
