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

/// How a step's answer shows in [`ROW_BY_ROW_REFUSALS`].
fn described(outcome: Result<RootOutcome, FileStop>) -> String {
    match outcome {
        Ok(RootOutcome::Complete(table)) => format!("complete {:?} {:?}", table.kinds, table.count),
        Ok(RootOutcome::Partial { table, .. }) => format!("partial {:?}", table.kinds),
        Ok(RootOutcome::Skipped(_)) => "skipped".into(),
        Ok(RootOutcome::Unknown(error)) => format!("unknown {error:?}"),
        Ok(RootOutcome::Failed(error)) => format!("failed {error:?}"),
        Ok(RootOutcome::NoQuery) => "noquery".into(),
        Ok(RootOutcome::Deferred { .. }) => "deferred".into(),
        Err(_) => "filestop".into(),
    }
}

/// Each operator that reads its operands row by row, over a column past its input or of the wrong kind (or a sound one, as a
/// control), by name.
fn row_by_row_steps() -> Vec<(&'static str, Operation)> {
    use ryo_values::domain::plan_parts::types::{CapturePattern, LiteralText, ParameterId};
    use ryo_values::domain::query_operators::types::{LineOutput, MaxLine, TabWidth, TextOutput};
    let from = Input::Step(StepId(0));
    let text = |written: &str| LiteralText(written.into());
    let matches = |span: u32| Operation::SourceMatches {
        from,
        span: ColumnIndex(span),
        base_line: None,
        pattern: text("x"),
        capture: None,
        capture_text: TextOutput::Omitted,
        capture_line: LineOutput::Omitted,
    };
    let render = |node: u32| Operation::RenderSyntax {
        from,
        node: ColumnIndex(node),
        context: ryo_language::domain::semantics::types::RenderContext::PositionalArgument,
    };
    let fits = |texts: u32, span: u32| Operation::FitsRendered {
        from,
        texts: vec![ColumnIndex(texts)],
        literals: vec![],
        span: ColumnIndex(span),
        max_line: MaxLine(80),
        tab_width: TabWidth(4),
    };
    let capture = |path: u32| Operation::Capture {
        from,
        path: ColumnIndex(path),
        pattern: CapturePattern("{x}".into()),
    };
    let replace = |column: u32, pattern: &str| Operation::Replace {
        from,
        text: ColumnIndex(column),
        pattern: text(pattern),
        replacement: text("b"),
    };
    vec![
        ("capture-col99", capture(99)),
        ("capture-num", capture(1)),
        (
            "rank-col99",
            Operation::Rank {
                from,
                column: ColumnIndex(99),
                sets: ParameterId(0),
            },
        ),
        (
            "words-col99",
            Operation::Words {
                from,
                text: ColumnIndex(99),
            },
        ),
        (
            "words-num",
            Operation::Words {
                from,
                text: ColumnIndex(1),
            },
        ),
        ("replace-col99", replace(99, "a")),
        ("replace-num", replace(1, "a")),
        ("replace-badre", replace(0, "(")),
        (
            "spanbetween-col99",
            Operation::SpanBetween {
                from,
                start: ColumnIndex(99),
                end: ColumnIndex(0),
            },
        ),
        (
            "spanbetween-text",
            Operation::SpanBetween {
                from,
                start: ColumnIndex(0),
                end: ColumnIndex(0),
            },
        ),
        ("sourcematches-col99", matches(99)),
        ("sourcematches-text", matches(0)),
        ("render-col99", render(99)),
        ("render-text", render(0)),
        ("fits-col99", fits(99, 1)),
        ("fits-spantext", fits(0, 0)),
        (
            "spanbounds-col99",
            Operation::SpanBounds {
                from,
                span: ColumnIndex(99),
            },
        ),
        (
            "spanbounds-text",
            Operation::SpanBounds {
                from,
                span: ColumnIndex(0),
            },
        ),
    ]
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
    use ryo_relations::domain::relations::types::Integer;
    let mut answered = String::new();
    for (name, operation) in row_by_row_steps() {
        for populated in [false, true] {
            for downstream in [false, true] {
                let mut input = authored_request(1);
                input.hosted = vec![HostRelation {
                    name: HostName("text".into()),
                    columns: vec![
                        HostColumn {
                            name: HostColumnName("value".into()),
                            kind: HostKind::Text,
                        },
                        HostColumn {
                            name: HostColumnName("n".into()),
                            kind: HostKind::Number,
                        },
                    ],
                    rows: if populated {
                        vec![HostRow(vec![
                            HostCell::Text("x".into()),
                            HostCell::Number(Integer(1)),
                        ])]
                    } else {
                        vec![]
                    },
                }];
                let mut steps = vec![
                    step(Operation::Hosted {
                        relation: HostedNumber(0),
                    }),
                    step(operation.clone()),
                ];
                if downstream {
                    steps.push(step(Operation::Project {
                        from: Input::Step(StepId(1)),
                        columns: vec![ColumnIndex(0)],
                        distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
                    }));
                }
                let emitted = StepId(u32::try_from(steps.len() - 1).expect("a few steps"));
                boundary_fixture::fixtures::plans(&mut input)[0].steps = steps;
                boundary_fixture::fixtures::plans(&mut input)[0].emit.step = emitted;
                let outcome = described(evaluate(input, &mut TableFacts::new(&[])));
                answered.push_str(&format!(
                    "{name} populated={populated} downstream={downstream}: {outcome}\n"
                ));
            }
        }
    }
    assert_eq!(answered, ROW_BY_ROW_REFUSALS);
}
