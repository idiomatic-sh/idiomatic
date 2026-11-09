//! Exact physical spans, CRLF lines, capture refusals and interruption prefixes.
use super::service::occurrences;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::errors::ContractError;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::source_matches::types::{
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
type Failure = PortableOperatorFault;
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
        expect: Expect::Error(PortableOperatorFault::MissingCapture),
    },
    Case {
        name: CaseName("first_sink_failure_stops_before_next_capture"),
        word: Word::Refuses,
        input: || Scenario::Interrupted,
        expect: Expect::Error(PortableOperatorFault::Interrupted),
    },
    Case {
        name: CaseName("unrepresentable_absolute_span_refuses"),
        word: Word::Refuses,
        input: || Scenario::Overflow,
        expect: Expect::Error(PortableOperatorFault::InvalidSourceMatchSpan),
    },
    Case {
        name: CaseName("capture_outputs_require_named_capture"),
        word: Word::Refuses,
        input: || Scenario::MissingOptions,
        expect: Expect::Error(PortableOperatorFault::WrongOperation),
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
        _ => (0, Some(1), Some("cap")),
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
                .map_err(|_| PortableOperatorFault::MissingCapture),
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
        _ => {
            assert_eq!(driver.interns, 0);
            assert_eq!(driver.deliveries, 0);
        }
    }
    result?;
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
