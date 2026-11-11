//! Named producer populations use the actual prepared union and common file scheduler.
use super::fixtures::{Emitted, Observed, exercise};
use ryo_values::domain::plan_parts::types::StepScope;
use std::borrow::Cow;
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = &'static [&'static [i64]];
type Value = Observed;
type Failure = String;

/// A union of `numbers`, dispatched `registered_calls` times, replayed unchanged.
const fn union(
    producers_complete: usize,
    numbers: &'static [i64],
    registered_calls: usize,
) -> Observed {
    Observed {
        producers_complete,
        emitted: Emitted::Union {
            numbers: Cow::Borrowed(numbers),
            rows: numbers.len(),
        },
        registered_calls,
        replayed: Emitted::Union {
            numbers: Cow::Borrowed(numbers),
            rows: numbers.len(),
        },
        registered_unchanged: true,
        readiness_unchanged: true,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "singleton_and_many_producers_preserve_distinct_union_order_through_one_prepared_root",
        ),
        word: Word::Succeeds,
        input: || &[&[1, 2], &[2, 3], &[1, 3]],
        expect: Expect::Value(union(3, &[1, 2, 3], 5)),
    },
    Case {
        name: CaseName("a_singleton_producer_of_no_rows_unions_to_nothing"),
        word: Word::Succeeds,
        input: || &[&[]],
        expect: Expect::Value(union(1, &[], 0)),
    },
    Case {
        name: CaseName("a_singleton_producer_unions_to_its_distinct_rows"),
        word: Word::Succeeds,
        input: || &[&[1, 1, 2]],
        expect: Expect::Value(union(1, &[1, 2], 6)),
    },
    Case {
        name: CaseName("a_producer_before_an_empty_one_unions_to_its_distinct_rows"),
        word: Word::Succeeds,
        input: || &[&[1, 1, 2], &[]],
        expect: Expect::Value(union(2, &[1, 2], 3)),
    },
    Case {
        name: CaseName("a_producer_after_an_empty_one_unions_to_its_distinct_rows"),
        word: Word::Succeeds,
        input: || &[&[], &[1, 1, 2]],
        expect: Expect::Value(union(2, &[1, 2], 3)),
    },
    Case {
        name: CaseName("zero_producers_preserve_the_declared_project_phase_in_file_scope"),
        word: Word::Succeeds,
        input: || &[],
        expect: Expect::Value(Observed {
            producers_complete: 0,
            emitted: Emitted::Deferred(StepScope::CrossFile),
            registered_calls: 0,
            replayed: Emitted::Deferred(StepScope::CrossFile),
            registered_unchanged: true,
            readiness_unchanged: true,
        }),
    },
];

/// Runs the producers of the given populations and the consumer of their union.
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(exercise(input))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
