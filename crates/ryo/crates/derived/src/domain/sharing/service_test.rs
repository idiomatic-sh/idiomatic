//! The `sharing` service's cases, on values alone.

use super::service::{
    downgrade_positions, marks_capacity, marks_key, positions_key, upgrade_positions,
};
use super::types::{SharedMarks, SharedPositions};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Scenario {
    CloneKeepsIdentity,
    WeakHandleLivesWithOwner,
    WeakHandleEndsWithOwner,
    MarksCapacityCoversTheMarks,
    MarksIdentityIsTheirAllocation,
}
type Input = Scenario;
type Value = usize;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_clone_shares_the_allocation_identity"),
        word: Word::Succeeds,
        input: || Scenario::CloneKeepsIdentity,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_weak_handle_upgrades_while_an_owner_holds_the_rows"),
        word: Word::Succeeds,
        input: || Scenario::WeakHandleLivesWithOwner,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_weak_handle_ends_with_the_last_owner"),
        word: Word::Succeeds,
        input: || Scenario::WeakHandleEndsWithOwner,
        expect: Expect::Value(0),
    },
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
    let owner = SharedPositions(Arc::new(vec![2, 5]));
    let marks = SharedMarks(Arc::new(vec![true, false]));
    Ok(match input {
        Scenario::CloneKeepsIdentity => {
            usize::from(positions_key(&owner.clone()) == positions_key(&owner))
        }
        Scenario::WeakHandleLivesWithOwner => {
            upgrade_positions(&downgrade_positions(&owner)).map_or(0, |rows| rows.0.len())
        }
        Scenario::WeakHandleEndsWithOwner => {
            let weak = downgrade_positions(&owner);
            drop(owner);
            upgrade_positions(&weak).map_or(0, |rows| rows.0.len())
        }
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
