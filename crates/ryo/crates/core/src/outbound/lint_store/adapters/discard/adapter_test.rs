//! The discard adapter of the `lint_store` port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::LintStoreDiscardAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::fixtures::contract::check;
use crate::outbound::lint_store::fixtures::tables::stored;
use crate::outbound::lint_store::interface::LintStorePort as _;
use crate::outbound::lint_store::types::StoreKeeping;
use ryo_values::domain::stores::types::{KeptRows, Stored};

/// What a row of the cases asks of the adapter.
#[derive(Clone, Copy)]
enum Scenario {
    /// Says it keeps nothing, is given tables and reads back as empty.
    KeepsNothing,
}

/// What a row of the cases asks of the adapter.
type Input = Scenario;
/// Whether the adapter did what the row asks.
type Value = bool;
type Failure = LintStoreError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("it_says_it_keeps_nothing_and_keeps_nothing"),
    word: Word::Succeeds,
    input: || Scenario::KeepsNothing,
    expect: Expect::Value(true),
}];

/// Calls the adapter on one input, for its own rows.
fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::KeepsNothing => {
            let store = LintStoreDiscardAdapter;
            store.stored()?.keep(stored(1), &KeptRows::default())?;
            Ok(store.keeps() == StoreKeeping::Discards
                && store.stored()?.tables() == Stored::default())
        }
    }
}

#[test]
fn succeeds() {
    check(&|| Box::new(LintStoreDiscardAdapter));
    run(CASES, Word::Succeeds, subject);
}
