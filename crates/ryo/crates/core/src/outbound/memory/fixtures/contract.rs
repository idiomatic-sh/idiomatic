//! What every `memory` adapter must pass.

use super::super::errors::MemoryError;
use super::super::interface::MemoryPort;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract asks.
#[derive(Clone, Copy, Debug)]
pub enum Asked {
    /// The memory to spare.
    Available,
    /// The memory the process holds.
    Resident,
}

/// What a row of the contract is given.
type Input = Asked;
/// Whether there is memory to spare.
type Value = bool;
/// What a row of the contract expects when it is refused.
type Failure = MemoryError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("some_memory_is_available"),
        word: Word::Succeeds,
        input: || Asked::Available,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_process_holds_some_memory"),
        word: Word::Succeeds,
        input: || Asked::Resident,
        expect: Expect::Value(true),
    },
];

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl MemoryPort) {
    run(CASES, Word::Succeeds, |asked| {
        match asked {
            Asked::Available => adapter.budget(),
            Asked::Resident => adapter.resident_bytes(),
        }
        .map(|bytes| bytes.0 > 0)
    });
}
