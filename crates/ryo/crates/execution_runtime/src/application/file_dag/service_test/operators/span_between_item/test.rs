//! Exact byte offsets, absence and refusal through actual compiled owner execution.
use super::super::fixtures as compiled;
use super::fixtures::span;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    constants::NO_SPAN,
    types::{FileId, Relation, Span, Value as CellValue},
};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::{ColumnIndex, Input as PlanInput, JoinColumn, Side, StepId},
    plans::types::Operation,
};

/// The left and right spans of one node, and the file's source.
type Input = (Span, Span, &'static [u8]);
/// The span between, as each row answers it; the rows answered; whether the first row keeps the
/// left and right spans before it; whether the second row repeats the first; and whether the
/// streamed run answered the collected run's table.
#[derive(Debug, PartialEq)]
struct Between {
    span: Span,
    rows: usize,
    inputs_kept: bool,
    rows_alike: bool,
    streamed_same: bool,
}
/// The refusal, and whether the streamed run refused alike.
#[derive(Debug, PartialEq)]
struct Refused {
    error: OperatingError,
    streamed_same: bool,
}
type Value = Between;
type Failure = Refused;

/// Two rows, each the left and right spans and `span` between them, streamed as collected.
const fn between(span: Span) -> Between {
    Between {
        span,
        rows: 2,
        inputs_kept: true,
        rows_alike: true,
        streamed_same: true,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("suffix_contained_in_boolop"),
        word: Word::Succeeds,
        input: || (span(5, 6), span(0, 11), b"x or 1 or y"),
        expect: Expect::Value(between(span(5, 11))),
    },
    Case {
        name: CaseName("overlap"),
        word: Word::Succeeds,
        input: || (span(1, 5), span(3, 8), b"0123456789"),
        expect: Expect::Value(between(span(1, 8))),
    },
    Case {
        name: CaseName("nested_end_is_not_union"),
        word: Word::Succeeds,
        input: || (span(1, 9), span(3, 6), b"0123456789"),
        expect: Expect::Value(between(span(1, 6))),
    },
    Case {
        name: CaseName("gap"),
        word: Word::Succeeds,
        input: || (span(1, 2), span(7, 8), b"0123456789"),
        expect: Expect::Value(between(span(1, 8))),
    },
    Case {
        name: CaseName("equal"),
        word: Word::Succeeds,
        input: || (span(1, 3), span(1, 3), b"012"),
        expect: Expect::Value(between(span(1, 3))),
    },
    Case {
        name: CaseName("empty_file"),
        word: Word::Succeeds,
        input: || (span(0, 0), span(0, 0), b""),
        expect: Expect::Value(between(span(0, 0))),
    },
    Case {
        name: CaseName("eof"),
        word: Word::Succeeds,
        input: || (span(3, 3), span(3, 3), b"abc"),
        expect: Expect::Value(between(span(3, 3))),
    },
    Case {
        name: CaseName("unicode_multiline_byte_offsets"),
        word: Word::Succeeds,
        input: || (span(1, 2), span(3, 5), "é\nxy".as_bytes()),
        expect: Expect::Value(between(span(1, 5))),
    },
    Case {
        name: CaseName("absent_left"),
        word: Word::Succeeds,
        input: || (NO_SPAN, span(0, 1), b"a"),
        expect: Expect::Value(between(NO_SPAN)),
    },
    Case {
        name: CaseName("absent_right"),
        word: Word::Succeeds,
        input: || (span(0, 1), NO_SPAN, b"a"),
        expect: Expect::Value(between(NO_SPAN)),
    },
    Case {
        name: CaseName("both_absent"),
        word: Word::Succeeds,
        input: || (NO_SPAN, NO_SPAN, b""),
        expect: Expect::Value(between(NO_SPAN)),
    },
    Case {
        name: CaseName("absent_still_checks_right"),
        word: Word::Refuses,
        input: || (NO_SPAN, span(2, 1), b"ab"),
        expect: Expect::Error(Refused {
            error: OperatingError::InvalidSpan,
            streamed_same: true,
        }),
    },
    Case {
        name: CaseName("absent_still_checks_left"),
        word: Word::Refuses,
        input: || (span(2, 1), NO_SPAN, b"ab"),
        expect: Expect::Error(Refused {
            error: OperatingError::InvalidSpan,
            streamed_same: true,
        }),
    },
    Case {
        name: CaseName("reversed_composition"),
        word: Word::Refuses,
        input: || (span(7, 8), span(1, 2), b"0123456789"),
        expect: Expect::Error(Refused {
            error: OperatingError::InvalidSpan,
            streamed_same: true,
        }),
    },
    Case {
        name: CaseName("out_of_file"),
        word: Word::Refuses,
        input: || (span(0, 4), span(0, 1), b"abc"),
        expect: Expect::Error(Refused {
            error: OperatingError::InvalidSpan,
            streamed_same: true,
        }),
    },
    Case {
        name: CaseName("max_offset"),
        word: Word::Refuses,
        input: || (span(0, u32::MAX), span(0, 1), b"abc"),
        expect: Expect::Error(Refused {
            error: OperatingError::InvalidSpan,
            streamed_same: true,
        }),
    },
];

/// Runs the span between the two spans' syntax rows, collected and streamed, over the source.
fn subject((left, right, source): Input) -> Result<Value, Failure> {
    let steps = vec![
        compiled::select(Relation::SyntaxTree, &[ColumnIndex(0), ColumnIndex(4)]),
        compiled::select(Relation::SemanticSyntax, &[ColumnIndex(0), ColumnIndex(4)]),
        compiled::step(Operation::Join {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Step(StepId(1)),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(1),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(1),
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        }),
        compiled::step(Operation::SpanBetween {
            from: PlanInput::Step(StepId(2)),
            start: ColumnIndex(0),
            end: ColumnIndex(1),
        }),
    ];
    let providers = [
        compiled::Provider {
            relation: Relation::SyntaxTree,
            table: compiled::rows(
                &[compiled::FileSpan {
                    file: FileId(0),
                    span: left,
                }; 2],
            ),
        },
        compiled::Provider {
            relation: Relation::SemanticSyntax,
            table: compiled::rows(
                &[compiled::FileSpan {
                    file: FileId(0),
                    span: right,
                }; 2],
            ),
        },
    ];
    let compiled::CompiledRun {
        result: collected, ..
    } = compiled::run(
        steps.clone(),
        &providers,
        SourceView(source),
        &mut Symbols::default(),
        compiled::Downstream::Collected,
    );
    let compiled::CompiledRun {
        result: streamed, ..
    } = compiled::run(
        steps,
        &providers,
        SourceView(source),
        &mut Symbols::default(),
        compiled::Downstream::Streamed,
    );
    let streamed_same = collected == streamed;
    let output = collected.map_err(|error| Refused {
        error,
        streamed_same,
    })?;
    let CellValue::Span(span) = output.cells[2] else {
        panic!("span output required")
    };
    Ok(Between {
        span,
        rows: output.count.0,
        inputs_kept: output.cells[..2] == [CellValue::Span(left), CellValue::Span(right)],
        rows_alike: output.cells[3..] == output.cells[..3],
        streamed_same,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
