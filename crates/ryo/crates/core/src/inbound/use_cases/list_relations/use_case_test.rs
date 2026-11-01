//! What the `list_relations` use case promises, against the fake of the language port.

use std::sync::Arc;

use super::errors::ListRelationsError;
use super::interface::ListRelationsUseCase as _;
use super::use_case::ListRelationsUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::language::fixtures::fake::{FakeCatalog, LanguageFake};

/// The fake the use case is given.
type Input = fn() -> LanguageFake;
/// How many relations are listed.
type Value = usize;
type Failure = ListRelationsError;

fn working_catalog() -> LanguageFake {
    LanguageFake::default()
}

fn failing_catalog() -> LanguageFake {
    LanguageFake {
        catalog: FakeCatalog::Fails,
        ..LanguageFake::default()
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("lists_every_relation_ryo_owns"),
        word: Word::Succeeds,
        input: || working_catalog,
        expect: Expect::Value(45),
    },
    Case {
        name: CaseName("fails_when_language_port_fails"),
        word: Word::Fails,
        input: || failing_catalog,
        expect: Expect::Refusal(|error| selects(matches!(error, ListRelationsError::Language(..)))),
    },
];

fn subject(language: Input) -> Result<Value, Failure> {
    ListRelationsUseCaseImpl::new(Arc::new(language()))
        .list_relations()
        .map(|report| report.relations.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
