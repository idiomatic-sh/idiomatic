//! The `answers` service's cases, on values alone.

use super::service::{absent, source_locations};
use super::types::{Answer, Support};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::answers::types::AbsenceCause;

/// A scenario: what a row asks of the service.
type Input = fn() -> bool;
/// What a scenario answers: whether it holds.
type Value = bool;
type Failure = ();

fn a_relation_the_language_lacks_is_answered_with_its_reason() -> bool {
    let support: Support<()> = absent(AbsenceCause::NoSuchConstruct);
    matches!(
        support.as_ref(),
        Answer::NotInThisLanguage(AbsenceCause::NoSuchConstruct)
    )
}

fn the_engine_source_locations_are_a_reader() -> bool {
    matches!(source_locations::<()>().as_ref(), Answer::Reads(_))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_relation_the_language_lacks_is_answered_with_its_reason"),
        word: Word::Succeeds,
        input: || a_relation_the_language_lacks_is_answered_with_its_reason,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_engine_source_locations_are_a_reader"),
        word: Word::Succeeds,
        input: || the_engine_source_locations_are_a_reader,
        expect: Expect::Value(true),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
