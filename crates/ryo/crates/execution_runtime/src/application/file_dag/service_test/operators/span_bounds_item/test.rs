//! Endpoint projection through actual compiled owner execution.
use super::super::fixtures as compiled;
use super::fixtures::{evaluate, span};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    constants::NO_SPAN,
    types::{CellKind, FileId, Integer, Relation, Value as CellValue},
};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId},
    plans::types::Operation,
};
use std::borrow::Cow;
/// What the span bounds run over.
#[derive(Clone, Copy, Debug)]
enum Spans {
    /// One computed row per span.
    Rows(&'static [ryo_relations::domain::relations::types::Span]),
    /// Two syntax rows of files 1 and 2 at equal offsets 2..4.
    TwoFiles,
}

const fn number(value: i64) -> CellValue {
    CellValue::Number(Integer(value))
}

type Input = Spans;
/// The output's column kinds and cells, row after row, and (for computed rows) whether the
/// streamed run answered what the collected run did.
#[derive(Debug, PartialEq)]
struct Spanned {
    kinds: Cow<'static, [CellKind]>,
    cells: Cow<'static, [CellValue]>,
    streamed_same: Option<bool>,
}
/// The refusal, and (for computed rows) whether the streamed run refused alike.
#[derive(Debug, PartialEq)]
struct Refused {
    error: OperatingError,
    streamed_same: Option<bool>,
}
type Value = Spanned;
type Failure = Refused;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("checked_bounds_preserve_duplicate_rows_and_order"),
        word: Word::Succeeds,
        input: || Spans::Rows(const { &[span(2, 4), span(2, 4), span(0, 0)] }),
        expect: Expect::Value(Spanned {
            kinds: Cow::Borrowed(&[CellKind::Span, CellKind::Number, CellKind::Number]),
            cells: Cow::Borrowed(&[
                CellValue::Span(span(2, 4)),
                number(2),
                number(4),
                CellValue::Span(span(2, 4)),
                number(2),
                number(4),
                CellValue::Span(span(0, 0)),
                number(0),
                number(0),
            ]),
            streamed_same: Some(true),
        }),
    },
    Case {
        name: CaseName("absent_and_reversed_spans_refuse_before_row_output"),
        word: Word::Refuses,
        input: || Spans::Rows(const { &[NO_SPAN] }),
        expect: Expect::Error(Refused {
            error: OperatingError::InvalidSpanBounds,
            streamed_same: Some(true),
        }),
    },
    Case {
        name: CaseName("a_reversed_span_refuses_before_row_output"),
        word: Word::Refuses,
        input: || Spans::Rows(const { &[span(4, 3)] }),
        expect: Expect::Error(Refused {
            error: OperatingError::InvalidSpanBounds,
            streamed_same: Some(true),
        }),
    },
    Case {
        // Pure endpoint extraction does not claim the source bytes of a cross-file row.
        name: CaseName("a_span_past_the_source_is_bounded_without_its_bytes"),
        word: Word::Succeeds,
        input: || Spans::Rows(const { &[span(0, 7)] }),
        expect: Expect::Value(Spanned {
            kinds: Cow::Borrowed(&[CellKind::Span, CellKind::Number, CellKind::Number]),
            cells: Cow::Borrowed(&[CellValue::Span(span(0, 7)), number(0), number(7)]),
            streamed_same: Some(true),
        }),
    },
    Case {
        name: CaseName("equal_offsets_from_distinct_files_keep_both_file_identities"),
        word: Word::Succeeds,
        input: || Spans::TwoFiles,
        expect: Expect::Value(Spanned {
            kinds: Cow::Borrowed(&[
                CellKind::File,
                CellKind::Span,
                CellKind::Number,
                CellKind::Number,
            ]),
            cells: Cow::Borrowed(&[
                CellValue::File(FileId(1)),
                CellValue::Span(span(2, 4)),
                number(2),
                number(4),
                CellValue::File(FileId(2)),
                CellValue::Span(span(2, 4)),
                number(2),
                number(4),
            ]),
            streamed_same: None,
        }),
    },
];

/// Runs span bounds over the spans through the compiled owner.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Spans::Rows(spans) => evaluate(spans)
            .map(|evaluated| Spanned {
                kinds: Cow::Owned(evaluated.kinds),
                cells: Cow::Owned(evaluated.cells),
                streamed_same: Some(evaluated.streamed_same),
            })
            .map_err(|refusal| Refused {
                error: refusal.error,
                streamed_same: Some(refusal.streamed_same),
            }),
        Spans::TwoFiles => {
            let providers = [compiled::Provider {
                relation: Relation::SyntaxTree,
                table: compiled::rows(&[
                    compiled::FileSpan {
                        file: FileId(1),
                        span: span(2, 4),
                    },
                    compiled::FileSpan {
                        file: FileId(2),
                        span: span(2, 4),
                    },
                ]),
            }];
            let steps = vec![
                compiled::select(Relation::SyntaxTree, &[ColumnIndex(3), ColumnIndex(4)]),
                compiled::step(Operation::SpanBounds {
                    from: PlanInput::Step(StepId(0)),
                    span: ColumnIndex(1),
                }),
            ];
            let compiled::CompiledRun { result, .. } = compiled::run(
                steps,
                &providers,
                SourceView(b""),
                &mut Symbols::default(),
                compiled::Downstream::Collected,
            );
            result
                .map(|table| Spanned {
                    kinds: Cow::Owned(table.kinds),
                    cells: Cow::Owned(table.cells),
                    streamed_same: None,
                })
                .map_err(|error| Refused {
                    error,
                    streamed_same: None,
                })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
