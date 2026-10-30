//! What the schemas of this entry accept and refuse, one row of `CASES` for each input.

use ryo_core::inbound::use_cases::list_relations::errors::ListRelationsError as CoreError;
use ryo_core::outbound::language::errors::LanguageError;

use super::super::entry::list;
use super::super::errors::ListError;
use super::ListRelationsErrorSchema;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};

/// What a row offers the schemas.
enum Offered {
    /// Every relation, listed through the whole function.
    Listing,
    /// A failure of the core, offered to the conversion into the caller's.
    CoreFails(CoreError),
}

/// What a row gives the schemas.
type Input = Offered;
/// How many relations are listed without a name.
type Value = usize;
/// Why the schemas end in error.
type Failure = ListError;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_listed_relation_has_a_name"),
        word: Word::Succeeds,
        input: || Offered::Listing,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("fails_when_the_language_fails"),
        word: Word::Fails,
        input: || {
            Offered::CoreFails(CoreError::Language(LanguageError::Catalog {
                reason: String::new(),
            }))
        },
        expect: Expect::Refusal(|error| selects(matches!(error, ListError::Language { .. }))),
    },
];

/// Lists the relations and counts those whose name is blank, or converts the core's failure.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Offered::Listing => list().map(|report| {
            report
                .relations
                .iter()
                .filter(|each| each.name.0.trim().is_empty())
                .count()
        }),
        Offered::CoreFails(error) => Err(ListError::from(ListRelationsErrorSchema(error))),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
