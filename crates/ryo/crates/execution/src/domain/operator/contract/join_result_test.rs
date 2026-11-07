//! The `join_result` cases: error translation tests the production contract without an execution fixture.
use super::types::join_result;
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What a row gives: the kernel's terminal answer and the engine error recorded before it.
struct Terminal {
    /// The kernel's terminal answer.
    answer: Result<OperatorAnswer, PortableOperatorFault>,
    /// The engine error recorded before the answer.
    recorded: Option<OperatingError>,
}

type Input = Terminal;
type Value = ();
type Failure = OperatingError;

fn subject(terminal: Input) -> Result<Value, Failure> {
    join_result(terminal.answer, terminal.recorded)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_recorded_engine_error_wins_over_a_written_terminal"),
        word: Word::Refuses,
        input: || Terminal {
            answer: Ok(OperatorAnswer::Written),
            recorded: Some(OperatingError::NotRead),
        },
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("a_recorded_engine_error_wins_over_a_span_bounds_terminal"),
        word: Word::Refuses,
        input: || Terminal {
            answer: Ok(OperatorAnswer::SpanBounds([Cell::Number(Integer(1)); 2])),
            recorded: Some(OperatingError::NotRead),
        },
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("a_recorded_engine_error_wins_over_a_kernel_fault"),
        word: Word::Refuses,
        input: || Terminal {
            answer: Err(PortableOperatorFault::InvalidSpan),
            recorded: Some(OperatingError::NotRead),
        },
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("an_interruption_without_a_recorded_error_is_a_wrong_operation"),
        word: Word::Refuses,
        input: || Terminal {
            answer: Err(PortableOperatorFault::Interrupted),
            recorded: None,
        },
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("an_invalid_span_without_a_recorded_error_stays_invalid"),
        word: Word::Refuses,
        input: || Terminal {
            answer: Err(PortableOperatorFault::InvalidSpan),
            recorded: None,
        },
        expect: Expect::Error(OperatingError::InvalidSpan),
    },
    Case {
        name: CaseName("a_written_terminal_without_a_recorded_error_succeeds"),
        word: Word::Succeeds,
        input: || Terminal {
            answer: Ok(OperatorAnswer::Written),
            recorded: None,
        },
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
