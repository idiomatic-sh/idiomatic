//! The `sharing` service's cases, on values alone.

use super::service::{marks_capacity, marks_key};
use super::types::SharedMarks;
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Scenario {
    MarksCapacityCoversTheMarks,
    MarksIdentityIsTheirAllocation,
}
type Input = Scenario;
type Value = usize;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_capacity_of_marks_covers_them"),
        word: Word::Succeeds,
        input: || Scenario::MarksCapacityCoversTheMarks,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("marks_are_identified_by_their_allocation"),
        word: Word::Succeeds,
        input: || Scenario::MarksIdentityIsTheirAllocation,
        expect: Expect::Value(1),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let marks = SharedMarks(Arc::new(vec![true, false]));
    Ok(match input {
        Scenario::MarksCapacityCoversTheMarks => usize::from(marks_capacity(&marks).0 >= 2),
        Scenario::MarksIdentityIsTheirAllocation => {
            usize::from(marks_key(&marks.clone()) == marks_key(&marks))
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
