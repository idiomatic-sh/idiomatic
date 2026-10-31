//! Scalar columns, byte offsets and line-ending boundaries.
use super::schemas::offset;
use ryo_benchmark::domain::diagnostics::errors::Error;

#[derive(Debug, PartialEq, Eq)]
enum BoundaryFailure {
    Invalid,
    Unexpected,
}
fn failure(error: Error) -> BoundaryFailure {
    match error {
        Error::Invalid(_) => BoundaryFailure::Invalid,
        Error::Io(_) | Error::Json(_) | Error::Toml(_) => BoundaryFailure::Unexpected,
    }
}
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};

type Input = (&'static str, usize, usize);
type Value = usize;
type Failure = BoundaryFailure;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unicode_scalar_is_not_a_byte"),
        word: Word::Succeeds,
        input: || ("é = 1\r\npass\n", 1, 2),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("crlf_counts_as_one_line_break"),
        word: Word::Succeeds,
        input: || ("é = 1\r\npass\n", 2, 1),
        expect: Expect::Value(8),
    },
    Case {
        name: CaseName("exclusive_end_after_last_newline"),
        word: Word::Succeeds,
        input: || ("pass\n", 2, 1),
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("bom_is_excluded_from_first_scalar_column"),
        word: Word::Succeeds,
        input: || ("\u{feff}import os\n", 1, 8),
        expect: Expect::Value(10),
    },
    Case {
        name: CaseName("bom_does_not_shift_exclusive_end"),
        word: Word::Succeeds,
        input: || ("\u{feff}import os\n", 1, 10),
        expect: Expect::Value(12),
    },
    Case {
        name: CaseName("lone_carriage_return_ends_a_line"),
        word: Word::Succeeds,
        input: || ("x = 1\rtry:\r    pass\rexcept:\r    pass\r", 4, 1),
        expect: Expect::Value(20),
    },
    Case {
        name: CaseName("mixed_line_endings"),
        word: Word::Succeeds,
        input: || ("one\r\ntwo\rthree\nfour", 4, 1),
        expect: Expect::Value(15),
    },
    Case {
        name: CaseName("zero_row_is_invalid"),
        word: Word::Refuses,
        input: || ("é", 0, 1),
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("zero_column_is_invalid"),
        word: Word::Refuses,
        input: || ("é", 1, 0),
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("row_outside_source_is_invalid"),
        word: Word::Refuses,
        input: || ("é", 2, 1),
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
    Case {
        name: CaseName("column_outside_source_is_invalid"),
        word: Word::Refuses,
        input: || ("é", 1, 5),
        expect: Expect::Error(BoundaryFailure::Invalid),
    },
];

fn subject((source, row, column): Input) -> Result<Value, Failure> {
    offset(source, row, column).map_err(failure)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
