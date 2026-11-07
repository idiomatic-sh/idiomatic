//! The language adapter of the catalog port: the port's contract rows run through it, then the cases specific to it.

use ryo_relations::domain::relations::types::Relation;

use super::adapter::CatalogLanguageAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;
use crate::outbound::catalog::errors::CatalogError;
use crate::outbound::catalog::fixtures::contract::check;
use crate::outbound::catalog::fixtures::fake::CatalogFake;
use crate::outbound::catalog::interface::CatalogPort as _;

/// What a row of the cases asks: the catalog of the adapter, or the failure the port names when a catalog cannot be read.
#[derive(Clone, Copy)]
enum Scenario {
    /// The adapter's catalog is asked for all of it.
    Listed,
    /// A catalog that cannot be read, as the port's fake stands in for it.
    Unreadable,
}

/// What a row of the cases asks.
type Input = Scenario;
/// Whether the catalog lists the `function` relation.
type Value = bool;
/// What a row expects when the catalog cannot be read.
type Failure = CatalogError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_catalog_lists_the_function_relation"),
        word: Word::Succeeds,
        input: || Scenario::Listed,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("fails_when_the_catalog_cannot_be_read"),
        word: Word::Fails,
        input: || Scenario::Unreadable,
        expect: Expect::Refusal(|error| selects!(matches!(error, CatalogError::Unavailable { .. }))),
    },
];

/// Asks the catalog for its relations and says whether it lists `function`.
fn subject(scenario: Input) -> Result<Value, Failure> {
    let relations = match scenario {
        Scenario::Listed => CatalogLanguageAdapter.relations()?,
        Scenario::Unreadable => CatalogFake {
            relations: Vec::new(),
            fails: true,
        }
        .relations()?,
    };
    Ok(relations.0.contains(&Relation::Function))
}

#[test]
fn succeeds() {
    check(&CatalogLanguageAdapter);
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
