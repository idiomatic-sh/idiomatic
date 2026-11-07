//! The memory adapter of the `lint_store` port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::LintStoreMemoryAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::fixtures::contract::check;
use crate::outbound::lint_store::fixtures::tables::stored;
use crate::outbound::lint_store::interface::LintStorePort as _;
use ryo_values::domain::stores::types::KeptRows;

/// What a row of the cases asks of the adapter.
#[derive(Clone, Copy)]
enum Scenario {
    /// Keeps tables in one adapter and reads another: what a process keeps is its own.
    AnotherAdapter,
    /// Keeps three times: the adapter keeps one store, and a keep replaces it whole.
    TheKeptStoreIsReplaced,
}

/// What a row of the cases asks of the adapter.
type Input = Scenario;
/// Whether the adapter did what the row asks.
type Value = bool;
type Failure = LintStoreError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("what_one_adapter_keeps_another_does_not_read"),
        word: Word::Succeeds,
        input: || Scenario::AnotherAdapter,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_kept_store_is_replaced_and_not_added_to"),
        word: Word::Succeeds,
        input: || Scenario::TheKeptStoreIsReplaced,
        expect: Expect::Value(true),
    },
];

/// Calls the adapter on one input, for its own rows.
fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::TheKeptStoreIsReplaced => {
            use super::constants::KEPT_STORE;
            use super::types::KeptStore;
            let adapter = LintStoreMemoryAdapter::default();
            for tag in [1, 2, 3] {
                adapter.stored()?.keep(stored(tag), &KeptRows::default())?;
            }
            let stores = [adapter.stored()?.tables()];
            Ok(KeptStore::default()
                .0
                .lock()
                .is_ok_and(|fresh| fresh.files.is_empty())
                && stores.len() == KEPT_STORE
                && stores[0].rules == stored(3).rules)
        }
        Scenario::AnotherAdapter => {
            let first = LintStoreMemoryAdapter::default();
            first.stored()?.keep(stored(1), &KeptRows::default())?;
            Ok(LintStoreMemoryAdapter::default().stored()?.tables()
                == ryo_values::domain::stores::types::Stored::default()
                && first.stored()?.tables()
                    == ryo_values::domain::stores::types::Stored {
                        handed: ryo_values::domain::stores::types::StoredHanded::default(),
                        ..stored(1)
                    })
        }
    }
}

#[test]
fn succeeds() {
    check(&|| Box::new(LintStoreMemoryAdapter::default()));
    run(CASES, Word::Succeeds, subject);
}
