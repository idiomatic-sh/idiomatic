//! The `answers` service's cases, on values alone.

use super::constants::reason_name;
use super::service::{is_key, vocabulary_of};
use super::types::AbsenceCause;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::collections::relations::defines::item::DefinesColumn;
use crate::domain::relations::collections::relations::function::item::FunctionColumn;
use crate::domain::relations::types::{Column, ColumnKeyness};
use crate::domain::terms::types::{Vocabulary, WordBlock};

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

fn every_reason_has_a_name_of_its_own() -> usize {
    let mut names: Vec<&str> = [
        AbsenceCause::NoSuchConstruct,
        AbsenceCause::NotCode,
        AbsenceCause::NotAKeyedDocument,
        AbsenceCause::NoManifests,
    ]
    .into_iter()
    .map(reason_name)
    .collect();
    names.sort_unstable();
    names.dedup();
    names.len()
}

/// The kind of a declaration is a closed set of words and its id is open; the id of a function is a key and its name is not.
fn a_column_has_its_vocabulary_and_its_keyness() -> usize {
    usize::from(
        vocabulary_of(Column::Defines(DefinesColumn::Kind))
            == Vocabulary::Closed(WordBlock::Declaration)
            && vocabulary_of(Column::Defines(DefinesColumn::Id)) == Vocabulary::Open
            && is_key(Column::Function(FunctionColumn::Id)) == ColumnKeyness::Key
            && is_key(Column::Defines(DefinesColumn::Kind)) == ColumnKeyness::Plain,
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_reason_has_a_name_of_its_own"),
        word: Word::Succeeds,
        input: || every_reason_has_a_name_of_its_own,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("a_column_has_its_vocabulary_and_its_keyness"),
        word: Word::Succeeds,
        input: || a_column_has_its_vocabulary_and_its_keyness,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
