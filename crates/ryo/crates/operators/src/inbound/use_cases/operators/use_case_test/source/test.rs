//! Public source-match calls keep matcher construction at the edge and preserve exact delivery prefixes.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::*;
use crate::domain::source_matches::types::*;
use crate::inbound::use_cases::operators::fixtures;
use crate::inbound::use_cases::operators::types::*;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use ryo_values::domain::row_texts::types::CellText;
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    SourceLines,
    SourceMissingCapture,
    SourceMissingOptions,
    SourceInterrupted,
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
];
fn source(input: Input) -> Result<Value, Failure> {
    let source = if matches!(input, Scenario::SourceInterrupted) {
        "a\r\nb\rc\nd"
    } else {
        "a\r\nb\rc"
    };
    let regex = fixtures::regex(if matches!(input, Scenario::SourceMissingCapture) {
        "(?P<cap>x)"
    } else {
        "(?P<cap>b|c|d)"
    })
    .unwrap();
    let capture = match input {
        Scenario::SourceMissingCapture => Some("absent"),
        Scenario::SourceMissingOptions => None,
        Scenario::SourceLines | Scenario::SourceInterrupted => Some("cap"),
    };
    let mut delivery = fixtures::SourceDelivery {
        fail_second: matches!(input, Scenario::SourceInterrupted),
        ..fixtures::SourceDelivery::default()
    };
    let mut output = vec![fixtures::number(99)];
    let result = fixtures::written(OperatorCall::SourceMatches {
        input: SourceMatchesInput {
            row: &[],
            source: LineBytes(source.as_bytes()),
            base: SourceBase(0),
            text: CellText(source),
            line: BaseLine::Starts(Integer(1)),
            regex: &regex,
            capture: capture.map_or(CaptureRequest::Whole, |name| {
                CaptureRequest::Group(CellText(name))
            }),
            capture_text: TextOutput::Appended,
            capture_line: LineOutput::Appended,
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
                        fixtures::span(3, 4),
                        fixtures::number(2),
                        fixtures::span(3, 4),
                        Cell::Symbol(Symbol(1)),
                        fixtures::number(2)
                    ],
                    vec![
                        fixtures::span(5, 6),
                        fixtures::number(3),
                        fixtures::span(5, 6),
                        Cell::Symbol(Symbol(2)),
                        fixtures::number(3)
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
                    fixtures::span(3, 4),
                    fixtures::number(2),
                    fixtures::span(3, 4),
                    Cell::Symbol(Symbol(1)),
                    fixtures::number(2)
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
                vec![fixtures::number(99)],
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
    source(input)
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
