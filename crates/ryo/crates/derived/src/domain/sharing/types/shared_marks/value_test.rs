//! The shared marks keep what they were given and their allocation identity.
use super::value::SharedMarks;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::positions::types::Mark;

#[derive(Clone, Copy)]
enum Scenario {
    MarksInOrder,
    CloneKeepsIdentity,
}
type Input = Scenario;
type Value = u64;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_marks_keep_their_order"),
        word: Word::Succeeds,
        input: || Scenario::MarksInOrder,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_clone_shares_the_allocation_identity"),
        word: Word::Succeeds,
        input: || Scenario::CloneKeepsIdentity,
        expect: Expect::Value(1),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let marks = SharedMarks::new(vec![Mark::Member, Mark::Absent]);
    Ok(match input {
        Scenario::MarksInOrder => u64::from(marks.as_slice() == [Mark::Member, Mark::Absent]),
        Scenario::CloneKeepsIdentity => {
            u64::from(marks.clone().allocation_key() == marks.allocation_key())
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
