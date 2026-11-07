//! Complete and refused producer publication through the application service.
use super::{
    errors::PublicationError,
    fixtures::{Scenario, subject as fixture_subject},
    service::publication,
};
use crate::domain::finish::types::PublicationReason;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = Scenario;
type Value = ();
type Failure = PublicationError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(numeric_row_is_unchanged)),
        word: Word::Succeeds,
        input: || Scenario::Number,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(symbol_becomes_a_portable_name)),
        word: Word::Succeeds,
        input: || Scenario::Symbol,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(reserved_worker_refuses_without_mutation)),
        word: Word::Refuses,
        input: || Scenario::ReservedWorker,
        expect: Expect::Error(PublicationError::Publication {
            reason: PublicationReason::ReservedArena,
        }),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    fixture_subject(input, publication)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
