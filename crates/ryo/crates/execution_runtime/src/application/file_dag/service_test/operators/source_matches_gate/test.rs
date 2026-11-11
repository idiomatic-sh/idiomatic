//! Real compiled source/line gates precede the use of the matcher, which preparation compiled.
use super::fixtures::run as run_file;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::constants::NO_SPAN;
use ryo_relations::domain::relations::types::Span;
use super::super::fixtures::span;
/// A refusal: the gate's error in full, or the pattern the regex compiler refused.
#[derive(Debug, PartialEq)]
enum Refused {
    Gate(OperatingError),
    InvalidRegex,
}

/// The row's source span (none for no row), its base line, and whether the result is streamed.
type Input = (Option<Span>, i64, bool);
/// The regex builds counted.
type Value = usize;
/// The refusal and the regex builds counted.
type Failure = (Refused, usize);

// The source and line gates refuse a malformed row before its matcher is used: its error is the
// gate's, not the pattern's. The literal pattern is compiled once, eagerly, when the request is
// prepared (L-span L1), so every run counts that one build, and a row that passes the gates meets
// the compiled refusal.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("malformed_source_and_line_refuse_before_the_prepared_regex"),
        word: Word::Refuses,
        input: || (Some(span(0, 2)), 1, false),
        expect: Expect::Error((Refused::Gate(OperatingError::InvalidSourceMatchSpan), 1)),
    },
    Case {
        name: CaseName("a_line_before_the_first_is_refused_by_the_line_gate"),
        word: Word::Refuses,
        input: || (Some(span(0, 1)), 0, false),
        expect: Expect::Error((Refused::Gate(OperatingError::InvalidSourceMatchSpan), 1)),
    },
    Case {
        name: CaseName("a_row_without_a_span_passes_both_gates_unmatched"),
        word: Word::Succeeds,
        input: || (Some(NO_SPAN), 0, false),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("no_rows_pass_both_gates"),
        word: Word::Succeeds,
        input: || (None, 1, false),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_row_that_passes_the_gates_meets_the_compiled_refusal"),
        word: Word::Refuses,
        input: || (Some(span(0, 1)), 1, false),
        expect: Expect::Error((Refused::InvalidRegex, 1)),
    },
    Case {
        name: CaseName("malformed_source_and_line_refuse_before_the_prepared_regex_when_streamed"),
        word: Word::Refuses,
        input: || (Some(span(0, 2)), 1, true),
        expect: Expect::Error((Refused::Gate(OperatingError::InvalidSourceMatchSpan), 1)),
    },
    Case {
        name: CaseName("a_line_before_the_first_is_refused_by_the_line_gate_when_streamed"),
        word: Word::Refuses,
        input: || (Some(span(0, 1)), 0, true),
        expect: Expect::Error((Refused::Gate(OperatingError::InvalidSourceMatchSpan), 1)),
    },
    Case {
        name: CaseName("a_row_without_a_span_passes_both_gates_unmatched_when_streamed"),
        word: Word::Succeeds,
        input: || (Some(NO_SPAN), 0, true),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("no_rows_pass_both_gates_when_streamed"),
        word: Word::Succeeds,
        input: || (None, 1, true),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_row_that_passes_the_gates_meets_the_compiled_refusal_when_streamed"),
        word: Word::Refuses,
        input: || (Some(span(0, 1)), 1, true),
        expect: Expect::Error((Refused::InvalidRegex, 1)),
    },
];

/// Runs the select, extend and source match over the row.
fn subject((span, line, stream): Input) -> Result<Value, Failure> {
    let (result, builds) = run_file(span, line, stream);
    match result {
        Ok(()) => Ok(builds),
        Err(OperatingError::InvalidRegex { .. }) => Err((Refused::InvalidRegex, builds)),
        Err(error) => Err((Refused::Gate(error), builds)),
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
