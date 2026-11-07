//! Source-match calls with ordinary, line-less and span-only capture outputs keep the exact output shape.
use crate::domain::contract::types::*;
use crate::inbound::use_cases::operators::fixtures;
use crate::inbound::use_cases::operators::types::*;
use crate::domain::source_matches::types::*;
use ryo_relations::domain::relations::types::{Integer, Offset, Span, Value as Cell};
use ryo_relations::domain::terms::types::Symbol;
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use ryo_values::domain::row_texts::types::CellText;

#[derive(Clone, Copy)]
enum Scenario {
    SourceOrdinary,
    SourceOrdinaryInterrupted,
    SourceNoLine,
    SourceSpanOnly,
}
#[derive(Debug, PartialEq)]
enum Observation {
    Cells(Cow<'static, [Cell]>),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("source_ordinary_matches_keep_exact_rows_without_capture_outputs"),
        word: Word::Succeeds,
        input: || Scenario::SourceOrdinary,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            Cell::Number(Integer(7)),
            Cell::Span(Span {
                start: Offset(3),
                end: Offset(4),
            }),
            Cell::Number(Integer(7)),
            Cell::Span(Span {
                start: Offset(5),
                end: Offset(6),
            }),
        ]))),
    },
    Case {
        name: CaseName("source_ordinary_second_sink_refusal_keeps_prefix_and_stops"),
        word: Word::Fails,
        input: || Scenario::SourceOrdinaryInterrupted,
        expect: Expect::Error(Failure::Interrupted),
    },
    Case {
        name: CaseName("source_named_capture_without_line_metadata_keeps_exact_output_shape"),
        word: Word::Succeeds,
        input: || Scenario::SourceNoLine,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            Cell::Number(Integer(7)),
            Cell::Span(Span {
                start: Offset(3),
                end: Offset(4),
            }),
            Cell::Span(Span {
                start: Offset(3),
                end: Offset(4),
            }),
            Cell::Symbol(Symbol(1)),
            Cell::Number(Integer(7)),
            Cell::Span(Span {
                start: Offset(5),
                end: Offset(6),
            }),
            Cell::Span(Span {
                start: Offset(5),
                end: Offset(6),
            }),
            Cell::Symbol(Symbol(2)),
        ]))),
    },
    Case {
        name: CaseName("source_span_only_capture_omits_text_and_capture_line_outputs"),
        word: Word::Succeeds,
        input: || Scenario::SourceSpanOnly,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            Cell::Number(Integer(7)),
            Cell::Span(Span {
                start: Offset(3),
                end: Offset(4),
            }),
            Cell::Number(Integer(2)),
            Cell::Span(Span {
                start: Offset(3),
                end: Offset(4),
            }),
            Cell::Number(Integer(7)),
            Cell::Span(Span {
                start: Offset(5),
                end: Offset(6),
            }),
            Cell::Number(Integer(3)),
            Cell::Span(Span {
                start: Offset(5),
                end: Offset(6),
            }),
        ]))),
    },
];
fn source_optional(input: Input) -> Result<Value, Failure> {
    let interrupted = matches!(input, Scenario::SourceOrdinaryInterrupted);
    let ordinary = matches!(
        input,
        Scenario::SourceOrdinary | Scenario::SourceOrdinaryInterrupted
    );
    let source = if interrupted {
        "a\r\nb\rc\nd"
    } else {
        "a\r\nb\rc"
    };
    let regex = fixtures::regex("(?P<cap>b|c|d)");
    let mut delivery = fixtures::SourceDelivery {
        fail_second: interrupted,
        ..fixtures::SourceDelivery::default()
    };
    let mut output = Vec::new();
    let result = fixtures::written(OperatorCall::SourceMatches {
        input: SourceMatchesInput {
            row: &[Cell::Number(Integer(7))],
            source: LineBytes(source.as_bytes()),
            base: SourceBase(0),
            text: CellText(source),
            line: if matches!(input, Scenario::SourceSpanOnly) {
                BaseLine::Starts(Integer(1))
            } else {
                BaseLine::Unnumbered
            },
            regex: &regex,
            capture: if ordinary {
                CaptureRequest::Whole
            } else {
                CaptureRequest::Group(CellText("cap"))
            },
            capture_text: if matches!(input, Scenario::SourceNoLine) {
                TextOutput::Appended
            } else {
                TextOutput::Omitted
            },
            capture_line: LineOutput::Omitted,
        },
        driver: &mut delivery,
        target: &mut output,
    });
    if ordinary {
        let expected = if interrupted {
            vec![vec![fixtures::number(7), fixtures::span(3, 4)]]
        } else {
            vec![vec![fixtures::number(7), fixtures::span(3, 4)], vec![fixtures::number(7), fixtures::span(5, 6)]]
        };
        assert_eq!(
            delivery.rows, expected,
            "ordinary matches retain the original row and append only the whole span"
        );
        assert_eq!(
            delivery.attempts, 2,
            "second refusal prevents any third delivery"
        );
        assert_eq!(delivery.interns, 0);
        assert!(
            delivery.names.is_empty(),
            "ordinary find_iter never interns capture text"
        );
    } else if matches!(input, Scenario::SourceNoLine) {
        assert_eq!(
            delivery.rows,
            vec![
                vec![fixtures::number(7), fixtures::span(3, 4), fixtures::span(3, 4), Cell::Symbol(Symbol(1))],
                vec![fixtures::number(7), fixtures::span(5, 6), fixtures::span(5, 6), Cell::Symbol(Symbol(2))],
            ]
        );
        assert_eq!((delivery.attempts, delivery.interns), (2, 2));
        assert_eq!(delivery.names, ["b", "c"]);
    } else {
        assert_eq!(
            delivery.rows,
            vec![
                vec![fixtures::number(7), fixtures::span(3, 4), fixtures::number(2), fixtures::span(3, 4)],
                vec![fixtures::number(7), fixtures::span(5, 6), fixtures::number(3), fixtures::span(5, 6)],
            ]
        );
        assert_eq!((delivery.attempts, delivery.interns), (2, 0));
        assert!(
            delivery.names.is_empty(),
            "span-only captures never intern text"
        );
    }
    result?;
    Ok(Observation::Cells(Cow::Owned(
        delivery.rows.into_iter().flatten().collect(),
    )))
}
fn subject(input: Input) -> Result<Value, Failure> {
    source_optional(input)
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
