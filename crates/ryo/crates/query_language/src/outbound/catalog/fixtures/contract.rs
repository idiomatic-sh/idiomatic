//! What every `catalog` adapter must pass.

use super::super::errors::CatalogError;
use super::super::interface::CatalogPort;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract is given: nothing, the catalog is asked for all of it.
type Input = ();
/// Whether the catalog is well formed.
type Value = bool;
/// What a row of the contract expects when it is refused.
type Failure = CatalogError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_relation_has_columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl CatalogPort) {
    run(CASES, Word::Succeeds, |()| {
        let relations = adapter.relations()?;
        Ok(relations.0.iter().all(|relation| {
            !ryo_language::domain::relations::service::columns(*relation).is_empty()
        }))
    });
}
