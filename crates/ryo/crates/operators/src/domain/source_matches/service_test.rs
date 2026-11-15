//! Exact physical spans, CRLF lines, capture refusals and interruption prefixes.
use super::service::{absolute_span, occurrences};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::errors::ContractError;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::source_matches::types::{
    OccurrenceByte, OccurrenceSpan,
    BaseLine, CaptureRequest, LineBytes, SourceBase, SourceMatchesInput, TextRowDriver,
};
use ryo_matching::inbound::use_cases::matching::{
    interface::MatchingUseCase,
    types::{GroupName, RegexSource, SearchText},
    use_case::MatchingUseCaseImpl,
};
use ryo_relations::domain::relations::types::{Integer, Offset, Span, Value as Cell};
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use ryo_values::domain::row_texts::types::CellText;
#[derive(Clone, Copy)]
enum Scenario {
    Lines,
    MissingCapture,
    Interrupted,
    Overflow,
    MissingOptions,
}
type Input = Scenario;
type Value = std::borrow::Cow<'static, [Cell]>;
/// A refusal as a case states it: a fault, or a missing capture by its name.
#[derive(Debug, PartialEq)]
enum Refusal {
    /// Any other fault.
    Fault(PortableOperatorFault),
    /// The capture the pattern lacks.
    MissingCapture(std::borrow::Cow<'static, str>),
}
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("physical_crlf_and_cr_lines_track_whole_and_captured_spans"),
        word: Word::Succeeds,
        input: || Scenario::Lines,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            Cell::Span(Span {
                start: Offset(3),
                end: Offset(4),
            }),
            Cell::Number(Integer(2)),
            Cell::Span(Span {
                start: Offset(3),
                end: Offset(4),
            }),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(2)),
            Cell::Span(Span {
                start: Offset(5),
                end: Offset(6),
            }),
            Cell::Number(Integer(3)),
            Cell::Span(Span {
                start: Offset(5),
                end: Offset(6),
            }),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(3)),
        ])),
    },
    Case {
        name: CaseName("unknown_named_capture_refuses_before_intern_or_emit"),
        word: Word::Refuses,
        input: || Scenario::MissingCapture,
        expect: Expect::Error(Refusal::MissingCapture(std::borrow::Cow::Borrowed(
            "absent",
        ))),
    },
    Case {
        name: CaseName("first_sink_failure_stops_before_next_capture"),
        word: Word::Refuses,
        input: || Scenario::Interrupted,
        expect: Expect::Error(Refusal::Fault(PortableOperatorFault::Interrupted)),
    },
    Case {
        name: CaseName("unrepresentable_absolute_span_refuses"),
        word: Word::Refuses,
        input: || Scenario::Overflow,
        expect: Expect::Error(Refusal::Fault(
            PortableOperatorFault::InvalidSourceMatchSpan,
        )),
    },
    Case {
        name: CaseName("capture_outputs_require_named_capture"),
        word: Word::Refuses,
        input: || Scenario::MissingOptions,
        expect: Expect::Error(Refusal::Fault(PortableOperatorFault::WrongOperation)),
    },
];
struct Driver {
    scenario: Scenario,
    interns: usize,
    deliveries: usize,
    rows: Vec<Cell>,
}
impl TextRowDriver for Driver {
    fn intern(&mut self, text: CellText<'_>) -> Cell {
        self.interns += 1;
        assert!(text.0 == "b" || text.0 == "c");
        Cell::Number(Integer(i64::try_from(text.0.len()).unwrap()))
    }
    fn emit(&mut self, row: &[Cell]) -> Result<(), ContractError> {
        self.deliveries += 1;
        if matches!(self.scenario, Scenario::Interrupted) {
            return Err(ContractError::Interrupted);
        }
        self.rows.extend_from_slice(row);
        Ok(())
    }
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    let source = "a\r\nb\rc\nd";
    let regex = MatchingUseCaseImpl::new()
        .compile_regex(RegexSource("(?P<cap>b|c)"))
        .unwrap();
    let mut driver = Driver {
        scenario,
        interns: 0,
        deliveries: 0,
        rows: Vec::new(),
    };
    let (base, line, capture) = match scenario {
        Scenario::MissingCapture => (0, Some(1), Some("absent")),
        Scenario::Overflow => (usize::MAX, None, Some("cap")),
        Scenario::MissingOptions => (0, Some(1), None),
        Scenario::Lines | Scenario::Interrupted => (0, Some(1), Some("cap")),
    };
    let mut output = Vec::new();
    let result = occurrences(
        SourceMatchesInput {
            row: &[],
            source: LineBytes(source.as_bytes()),
            base: SourceBase(base),
            text: CellText(source),
            line: line.map_or(BaseLine::Unnumbered, |first| {
                BaseLine::Starts(Integer(first))
            }),
            regex: &regex,
            capture: capture.map_or(CaptureRequest::Whole, |name| {
                CaptureRequest::Group(CellText(name))
            }),
            capture_text: TextOutput::Appended,
            capture_line: if matches!(scenario, Scenario::Overflow) {
                LineOutput::Omitted
            } else {
                LineOutput::Appended
            },
        },
        &mut driver,
        &mut output,
        &|each| match capture {
            Some(name) => MatchingUseCaseImpl::new()
                .visit_regex_captures(
                    &regex,
                    SearchText(source),
                    GroupName(name),
                    &mut |found, group| each(found.0, Some(group.0)),
                )
                .map_err(|_| PortableOperatorFault::MissingCapture {
                    capture: ryo_values::domain::plan_parts::types::LiteralText(name.into()),
                }),
            None => Ok(MatchingUseCaseImpl::new().visit_regex_matches(
                &regex,
                SearchText(source),
                &mut |found| each(found.0, None),
            )),
        },
    );
    match scenario {
        Scenario::Lines => {
            assert_eq!(driver.interns, 2);
            assert_eq!(driver.deliveries, 2);
        }
        Scenario::Interrupted => {
            assert_eq!(driver.interns, 1);
            assert_eq!(driver.deliveries, 1);
            assert!(
                matches!(driver.emit(&[]), Err(ContractError::Interrupted)),
                "an interrupted delivery keeps refusing"
            );
        }
        Scenario::MissingCapture | Scenario::Overflow | Scenario::MissingOptions => {
            assert_eq!(driver.interns, 0);
            assert_eq!(driver.deliveries, 0);
        }
    }
    result.map_err(|fault| {
        if let PortableOperatorFault::MissingCapture { capture } = fault {
            Refusal::MissingCapture(std::borrow::Cow::Owned(capture.0.into()))
        } else {
            Refusal::Fault(fault)
        }
    })?;
    Ok(std::borrow::Cow::Owned(driver.rows))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// An occurrence at `start..end` of the text searched from `base`.
type Placed = (usize, usize, usize);

/// The absolute span of an occurrence: the one conversion the text_map stage's search makes of each whole and captured span.
const SPANS: &[Case<Placed, Span, PortableOperatorFault>] = &[
    Case {
        name: CaseName("an_occurrence_s_span_is_its_place_past_the_base"),
        word: Word::Succeeds,
        input: || (10, 3, 4),
        expect: Expect::Value(Span {
            start: Offset(13),
            end: Offset(14),
        }),
    },
    Case {
        name: CaseName("an_occurrence_ending_at_the_last_offset_a_span_holds_is_placed"),
        word: Word::Succeeds,
        input: || (4_294_967_293, 1, 2),
        expect: Expect::Value(Span {
            start: Offset(u32::MAX - 1),
            end: Offset(u32::MAX),
        }),
    },
    Case {
        name: CaseName("an_occurrence_ending_past_what_a_span_holds_refuses"),
        word: Word::Refuses,
        input: || (4_294_967_294, 1, 2),
        expect: Expect::Error(PortableOperatorFault::InvalidSourceMatchSpan),
    },
    Case {
        name: CaseName("an_occurrence_past_the_platform_s_offsets_refuses"),
        word: Word::Refuses,
        input: || (usize::MAX, 0, 1),
        expect: Expect::Error(PortableOperatorFault::InvalidSourceMatchSpan),
    },
];

fn placed((base, start, end): Placed) -> Result<Span, PortableOperatorFault> {
    absolute_span(
        SourceBase(base),
        OccurrenceSpan {
            start: OccurrenceByte(start),
            end: OccurrenceByte(end),
        },
    )
}

#[test]
fn spans_succeed() {
    run(SPANS, Word::Succeeds, placed);
}

#[test]
fn spans_refuse() {
    run(SPANS, Word::Refuses, placed);
}
