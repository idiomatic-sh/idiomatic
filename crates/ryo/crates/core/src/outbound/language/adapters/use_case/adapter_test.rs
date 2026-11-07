//! The `use_case` adapter of the language port: the port's contract rows run through it, then the cases specific to it.

use ryo_relations::domain::relations::service::all;

use super::adapter::LanguageUseCaseAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::language::errors::LanguageError;
use crate::outbound::language::fixtures::contract::check;
use crate::outbound::language::interface::LanguagePort as _;
use crate::outbound::language::types::ManifestReading;
use ryo_language::domain::files::types::FilePath;

/// What a row of the cases asks of the adapter.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// How many languages an adapter over none names.
    LinkedNames,
    /// Whether the catalog lists every relation ryo owns, whatever languages are linked.
    Catalog,
    /// Whether an adapter over no language reads a manifest.
    Manifest,
}

/// What a row of the cases asks of the adapter.
type Input = Scenario;
/// A count, or a yes or no as a number.
type Value = usize;
type Failure = LanguageError;

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_adapter_over_no_language_names_none"),
        word: Word::Succeeds,
        input: || Scenario::LinkedNames,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("the_catalog_lists_every_relation_ryo_owns_whatever_is_linked"),
        word: Word::Succeeds,
        input: || Scenario::Catalog,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_adapter_over_no_language_reads_no_manifest"),
        word: Word::Succeeds,
        input: || Scenario::Manifest,
        expect: Expect::Value(0),
    },
];

/// The adapter over no language.
fn linked_none() -> LanguageUseCaseAdapter {
    LanguageUseCaseAdapter::new()
}

/// Calls the adapter on one input, for its own rows.
fn subject(scenario: Input) -> Result<Value, Failure> {
    let adapter = linked_none();
    match scenario {
        Scenario::LinkedNames => Ok(adapter.linked().0.len()),
        Scenario::Catalog => Ok(usize::from(adapter.relations()?.0.len() == all().len())),
        Scenario::Manifest => Ok(usize::from(
            adapter.reads_manifest(&FilePath("Cargo.toml".into())) == ManifestReading::Read,
        )),
    }
}

#[test]
fn succeeds() {
    check(&linked_none());
    run(CASES, Word::Succeeds, subject);
}
