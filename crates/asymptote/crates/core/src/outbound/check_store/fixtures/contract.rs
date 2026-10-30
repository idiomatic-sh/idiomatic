//! What every `check_store` adapter must pass: what is kept is loaded back, and what is kept again replaces it.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::reports::types::{Moment, Recorded};
use crate::outbound::check_store::interface::CheckStorePort;
use crate::outbound::check_store::types::Stored;

/// How many times the store is kept before it is loaded.
type Input = u128;
/// Whether the load answered the last kept.
type Value = bool;
/// The rows only load, so none has an error to expect.
type Failure = ();

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("what_is_kept_is_loaded"),
        word: Word::Succeeds,
        input: || 1,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("what_is_kept_again_replaces_it"),
        word: Word::Succeeds,
        input: || 2,
        expect: Expect::Value(true),
    },
];

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl CheckStorePort) {
    run(CASES, Word::Succeeds, |times| -> Result<Value, Failure> {
        let mut last = Stored::default();
        for taken in 0..times {
            last = Stored {
                recorded: Recorded {
                    taken: Moment(taken),
                    ..Recorded::default()
                },
                ..Stored::default()
            };
            assert!(adapter.keep(&last).is_ok(), "what is given is kept");
        }
        Ok(adapter.load() == last)
    });
}
