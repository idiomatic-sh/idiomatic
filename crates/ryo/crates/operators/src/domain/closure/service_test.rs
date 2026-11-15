//! The `closure` service's cases, on values alone.

use super::service::lower;
use super::types::ReachParts;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// A scenario: whether what the part answers is what the case states.
type Input = fn() -> bool;
type Value = bool;
type Failure = ();

/// A closure reaches over its source and destination columns within the partition its keys name.
fn reaches_within_its_keys() -> bool {
    lower(&[ColumnIndex(3)], ColumnIndex(0), ColumnIndex(1))
        == ReachParts {
            keys: vec![ColumnIndex(3)],
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        }
}

/// A closure with no keys reaches over one partition.
fn unkeyed_is_one_partition() -> bool {
    lower(&[], ColumnIndex(1), ColumnIndex(0)).keys.is_empty()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_closure_reaches_within_its_keys"),
        word: Word::Succeeds,
        input: || reaches_within_its_keys,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_unkeyed_closure_is_one_partition"),
        word: Word::Succeeds,
        input: || unkeyed_is_one_partition,
        expect: Expect::Value(true),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(input())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
