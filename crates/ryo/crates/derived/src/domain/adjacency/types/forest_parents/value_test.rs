//! The ancestry hides its root sentinel behind absence.
use super::value::ForestParents;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::vertices::constants::FOREST_ROOT;
use crate::domain::vertices::types::VertexId;

#[derive(Clone, Copy)]
enum Scenario {
    ParentOfAChild,
    ParentOfARoot,
}
type Input = Scenario;
type Value = Option<u32>;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_child_has_its_parent"),
        word: Word::Succeeds,
        input: || Scenario::ParentOfAChild,
        expect: Expect::Value(Some(1)),
    },
    Case {
        name: CaseName("a_root_has_no_parent"),
        word: Word::Succeeds,
        input: || Scenario::ParentOfARoot,
        expect: Expect::Value(None),
    },
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let parents = [VertexId(1), FOREST_ROOT];
    let forest = ForestParents { parents: &parents };
    Ok(match input {
        Scenario::ParentOfAChild => forest.parent(VertexId(0)),
        Scenario::ParentOfARoot => forest.parent(VertexId(1)),
    }
    .map(|parent| parent.0))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
