//! What every `profile` adapter must pass.

use crate::outbound::profile::interface::ProfilePort;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract is given: which question to ask of the adapter.
type Input = Question;
/// What a row of the contract expects when it succeeds: whether the answer was the right one.
type Value = bool;
/// What a row of the contract expects when it is refused.
type Failure = ();

/// A question the contract asks of an adapter.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// A slot starts empty.
    StartsEmpty,
    /// What a visit leaves in the slot, the next visit finds.
    KeepsWhatIsLeft,
}

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_slot_starts_empty"),
        word: Word::Succeeds,
        input: || Question::StartsEmpty,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_slot_keeps_what_a_visit_leaves"),
        word: Word::Succeeds,
        input: || Question::KeepsWhatIsLeft,
        expect: Expect::Value(true),
    },
];

/// Asks one question of the adapter.
fn answer(adapter: &impl ProfilePort<u8>, question: Question) -> Value {
    let mut found = false;
    match question {
        Question::StartsEmpty => adapter.with_profile(&mut |profile| found = profile.is_none()),
        Question::KeepsWhatIsLeft => {
            adapter.with_profile(&mut |profile| *profile = Some(7));
            adapter.with_profile(&mut |profile| {
                found = *profile == Some(7);
                *profile = None;
            });
        }
    }
    found
}

/// Runs the contract's rows against one implementation: the fake, or an adapter of another crate.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl ProfilePort<u8>) {
    run(CASES, Word::Succeeds, |question| Ok(answer(adapter, question)));
}
