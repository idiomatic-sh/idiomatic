//! Public source-match calls keep matcher construction at the edge and preserve exact delivery prefixes.
use crate::inbound::use_cases::operators::{fixtures, interface::OperatorsUseCase};
use crate::domain::capture::types::*;
use crate::domain::contract::types::*;
use crate::domain::equality_join::types::*;
use crate::domain::ordered_existence::types::*;
use crate::domain::ordered_match::types::*;
use crate::domain::reduction::types::*;
use crate::domain::set_rows::types::*;
use crate::domain::source_matches::types::*;
use crate::domain::words::types::*;
use ryo_relations::domain::relations::types::{Integer, Offset, Span, Value as Cell};
use ryo_relations::domain::terms::types::Symbol;
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    SourceLines,
    SourceMissingCapture,
    SourceMissingOptions,
    SourceInterrupted,
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
        name: CaseName("source_dispatch_preserves_crlf_and_capture_line_numbers"),
        word: Word::Succeeds,
        input: || Scenario::SourceLines,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            Cell::Number(Integer(2)),
            Cell::Number(Integer(3)),
        ]))),
    },
    Case {
        name: CaseName("source_unknown_capture_fails_even_when_regex_matches_nothing"),
        word: Word::Refuses,
        input: || Scenario::SourceMissingCapture,
        expect: Expect::Error(Failure::MissingCapture),
    },
    Case {
        name: CaseName("source_capture_outputs_require_named_capture"),
        word: Word::Refuses,
        input: || Scenario::SourceMissingOptions,
        expect: Expect::Error(Failure::WrongOperation),
    },
    Case {
        name: CaseName("source_second_sink_failure_stops_later_interns_and_delivery"),
        word: Word::Fails,
        input: || Scenario::SourceInterrupted,
        expect: Expect::Error(Failure::Interrupted),
    },
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
fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}
fn written(call: OperatorCall<'_>) -> Result<(), Failure> {
    assert_eq!(
        fixtures::operators().operators(call)?,
        OperatorAnswer::Written
    );
    Ok(())
}
fn regex(pattern: &str) -> ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex {
    fixtures::regex(pattern)
}
fn span(start: u32, end: u32) -> Cell {
    Cell::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}
#[derive(Default)]
struct Delivery {
    rows: Vec<Vec<Cell>>,
    attempts: usize,
    interns: usize,
    names: Vec<String>,
    fail_second: bool,
}
impl TextRowDriver for Delivery {
    fn intern(&mut self, text: &str) -> Cell {
        self.interns += 1;
        assert!(
            matches!(text, "b" | "c" | "d"),
            "capture resolves the original physical text"
        );
        self.names.push(text.to_owned());
        Cell::Symbol(Symbol(u32::try_from(self.interns).unwrap()))
    }
    fn emit(&mut self, row: &[Cell]) -> Result<(), SemanticInterrupted> {
        self.attempts += 1;
        if self.fail_second && self.attempts == 2 {
            return Err(SemanticInterrupted);
        }
        self.rows.push(row.to_vec());
        Ok(())
    }
}
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
    let regex = regex("(?P<cap>b|c|d)");
    let mut delivery = Delivery {
        fail_second: interrupted,
        ..Delivery::default()
    };
    let mut output = Vec::new();
    let result = written(OperatorCall::SourceMatches {
        input: SourceMatchesInput {
            row: &[Cell::Number(Integer(7))],
            source: source.as_bytes(),
            base: 0,
            text: source,
            line: if matches!(input, Scenario::SourceSpanOnly) {
                Some(1)
            } else {
                None
            },
            regex: &regex,
            capture: if ordinary { None } else { Some("cap") },
            capture_text: matches!(input, Scenario::SourceNoLine),
            capture_line: false,
        },
        driver: &mut delivery,
        target: &mut output,
    });
    if ordinary {
        let expected = if interrupted {
            vec![vec![number(7), span(3, 4)]]
        } else {
            vec![vec![number(7), span(3, 4)], vec![number(7), span(5, 6)]]
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
                vec![number(7), span(3, 4), span(3, 4), Cell::Symbol(Symbol(1))],
                vec![number(7), span(5, 6), span(5, 6), Cell::Symbol(Symbol(2))],
            ]
        );
        assert_eq!((delivery.attempts, delivery.interns), (2, 2));
        assert_eq!(delivery.names, ["b", "c"]);
    } else {
        assert_eq!(
            delivery.rows,
            vec![
                vec![number(7), span(3, 4), number(2), span(3, 4)],
                vec![number(7), span(5, 6), number(3), span(5, 6)],
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
fn source(input: Input) -> Result<Value, Failure> {
    let source = if matches!(input, Scenario::SourceInterrupted) {
        "a\r\nb\rc\nd"
    } else {
        "a\r\nb\rc"
    };
    let regex = regex(if matches!(input, Scenario::SourceMissingCapture) {
        "(?P<cap>x)"
    } else {
        "(?P<cap>b|c|d)"
    });
    let capture = match input {
        Scenario::SourceMissingCapture => Some("absent"),
        Scenario::SourceMissingOptions => None,
        _ => Some("cap"),
    };
    let mut delivery = Delivery {
        fail_second: matches!(input, Scenario::SourceInterrupted),
        ..Delivery::default()
    };
    let mut output = vec![number(99)];
    let result = written(OperatorCall::SourceMatches {
        input: SourceMatchesInput {
            row: &[],
            source: source.as_bytes(),
            base: 0,
            text: source,
            line: Some(1),
            regex: &regex,
            capture,
            capture_text: true,
            capture_line: true,
        },
        driver: &mut delivery,
        target: &mut output,
    });
    match input {
        Scenario::SourceLines => {
            assert_eq!(
                delivery.rows,
                vec![
                    vec![
                        span(3, 4),
                        number(2),
                        span(3, 4),
                        Cell::Symbol(Symbol(1)),
                        number(2)
                    ],
                    vec![
                        span(5, 6),
                        number(3),
                        span(5, 6),
                        Cell::Symbol(Symbol(2)),
                        number(3)
                    ]
                ]
            );
            assert_eq!((delivery.attempts, delivery.interns), (2, 2));
            assert_eq!(delivery.names, ["b", "c"]);
        }
        Scenario::SourceInterrupted => {
            assert_eq!(
                delivery.rows,
                vec![vec![
                    span(3, 4),
                    number(2),
                    span(3, 4),
                    Cell::Symbol(Symbol(1)),
                    number(2)
                ]]
            );
            assert_eq!(
                (delivery.attempts, delivery.interns),
                (2, 2),
                "failure prevents the third capture and downstream delivery"
            );
            assert_eq!(delivery.names, ["b", "c"]);
        }
        Scenario::SourceMissingCapture | Scenario::SourceMissingOptions => {
            assert!(delivery.rows.is_empty());
            assert_eq!((delivery.attempts, delivery.interns), (0, 0));
            assert_eq!(
                output,
                vec![number(99)],
                "option faults precede output mutation"
            );
        }
        _ => unreachable!(),
    }
    result?;
    Ok(Observation::Cells(Cow::Owned(
        delivery.rows.iter().map(|row| row[1]).collect(),
    )))
}
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::SourceLines
        | Scenario::SourceMissingCapture
        | Scenario::SourceMissingOptions
        | Scenario::SourceInterrupted => source(input),
        Scenario::SourceOrdinary
        | Scenario::SourceOrdinaryInterrupted
        | Scenario::SourceNoLine
        | Scenario::SourceSpanOnly => source_optional(input),
    }
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
