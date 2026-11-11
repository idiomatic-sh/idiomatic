//! Ordered graph traversals that refuse name the exact error.
use super::fixtures::{
    distinct_orders_are_not_ties, duplicate_edges_are_ties, empty_input_kind_mismatch_is_rejected,
    malformed_empty_input_is_rejected, ties_are_rejected_even_without_roots,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::selects;

/// The traversal a row runs.
type Input = fn() -> Result<usize, OperatingError>;
/// How many rows the traversal answered.
type Value = usize;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("distinct_orders_from_one_root_are_not_ties"),
        word: Word::Succeeds,
        input: || distinct_orders_are_not_ties,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("ties_refused_even_with_empty_roots"),
        word: Word::Refuses,
        input: || ties_are_rejected_even_without_roots,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, OperatingError::DuplicateDfsOrder))
        }),
    },
    Case {
        name: CaseName("duplicate_edges_refused_as_order_ties"),
        word: Word::Refuses,
        input: || duplicate_edges_are_ties,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, OperatingError::DuplicateDfsOrder))
        }),
    },
    Case {
        name: CaseName("missing_columns_refused_on_empty_input"),
        word: Word::Refuses,
        input: || malformed_empty_input_is_rejected,
        expect: Expect::Refusal(|error| selects!(matches!(error, OperatingError::Table(..)))),
    },
    Case {
        name: CaseName("entity_kinds_checked_on_empty_input"),
        word: Word::Refuses,
        input: || empty_input_kind_mismatch_is_rejected,
        expect: Expect::Refusal(|error| selects!(matches!(error, OperatingError::ShapeMismatch))),
    },
];

/// Runs the row's traversal.
fn subject(input: Input) -> Result<Value, Failure> {
    super::super::fixtures::subject(input)
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
