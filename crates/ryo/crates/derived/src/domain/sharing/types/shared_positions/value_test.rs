//! The shared positions keep their allocation identity and release it when no owner holds it.
use super::value::SharedPositions;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::positions::types::RowSlot;

#[derive(Clone, Copy)]
enum Scenario {
    CloneKeepsIdentity,
    WeakHandleLivesWithOwner,
    WeakHandleEndsWithOwner,
}
type Input = Scenario;
type Value = u64;
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
];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let owner = SharedPositions::new(vec![RowSlot(2), RowSlot(5)]);
    Ok(match input {
        Scenario::CloneKeepsIdentity => {
            u64::from(owner.clone().allocation_key() == owner.allocation_key())
        }
        Scenario::WeakHandleLivesWithOwner => {
            let weak = owner.downgrade();
            weak.upgrade().map_or(0, |rows| rows.as_slice().len() as u64)
        }
        Scenario::WeakHandleEndsWithOwner => {
            let weak = owner.downgrade();
            drop(owner);
            weak.upgrade().map_or(0, |rows| rows.as_slice().len() as u64)
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
