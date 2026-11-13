//! Ordered graph traversals that refuse name the exact error.
use super::super::fixtures::Ran;
use super::fixtures::{
    distinct_orders_are_not_ties, duplicate_edges_are_ties, empty_input_kind_mismatch_is_rejected,
    malformed_empty_input_is_rejected, ties_are_rejected_even_without_roots,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::accounting::ledger::types::ByteCount;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::selects;

/// The traversal a row runs.
type Input = fn() -> Result<Ran, OperatingError>;
/// How many rows the traversal answered, and the working bytes left after the file finished.
#[derive(Debug, PartialEq)]
struct Answer {
    count: usize,
    retained: ByteCount,
}
/// The refusal, and the working bytes left after the file finished when the run reached its end.
#[derive(Debug, PartialEq)]
struct Refused {
    error: OperatingError,
    retained: Option<ByteCount>,
}
type Value = Answer;
type Failure = Refused;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("distinct_orders_from_one_root_are_not_ties"),
        word: Word::Succeeds,
        input: || distinct_orders_are_not_ties,
        expect: Expect::Value(Answer {
            count: 2,
            retained: ByteCount(0),
        }),
    },
    Case {
        name: CaseName("ties_refused_even_with_empty_roots"),
        word: Word::Refuses,
        input: || ties_are_rejected_even_without_roots,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                Refused {
                    error: OperatingError::DuplicateDfsOrder,
                    retained: Some(ByteCount(0))
                }
            ))
        }),
    },
    Case {
        name: CaseName("duplicate_edges_refused_as_order_ties"),
        word: Word::Refuses,
        input: || duplicate_edges_are_ties,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                Refused {
                    error: OperatingError::DuplicateDfsOrder,
                    retained: Some(ByteCount(0))
                }
            ))
        }),
    },
    Case {
        name: CaseName("missing_columns_refused_on_empty_input"),
        word: Word::Refuses,
        input: || malformed_empty_input_is_rejected,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                Refused {
                    error: OperatingError::Table(..),
                    retained: Some(ByteCount(0))
                }
            ))
        }),
    },
    Case {
        name: CaseName("entity_kinds_checked_on_empty_input"),
        word: Word::Refuses,
        input: || empty_input_kind_mismatch_is_rejected,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                Refused {
                    error: OperatingError::ShapeMismatch,
                    retained: Some(ByteCount(0))
                }
            ))
        }),
    },
];

/// Runs the row's traversal.
fn subject(input: Input) -> Result<Value, Failure> {
    let ran = input().map_err(|error| Refused {
        error,
        retained: None,
    })?;
    match ran.answer {
        Ok(table) => Ok(Answer {
            count: table.count.0,
            retained: ran.retained,
        }),
        Err(error) => Err(Refused {
            error,
            retained: Some(ran.retained),
        }),
    }
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
