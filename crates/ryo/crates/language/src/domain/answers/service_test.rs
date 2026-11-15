//! The `answers` service's cases, on values alone.

use super::service::{
    absent, expanded_semantic_child, expanded_semantic_syntax, native_semantic_child,
    native_semantic_syntax, source_locations,
};
use super::types::{Answer, Expansions, Reads, Support};
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

fn the_expanded_semantic_readers_look_at_added_elements() -> bool {
    let syntax = matches!(
        expanded_semantic_syntax::<()>().as_ref(),
        Answer::Reads(reader) if reader.expansions == Expansions::Included && matches!(reader.reads, Reads::Every)
    );
    let child = matches!(
        expanded_semantic_child::<()>().as_ref(),
        Answer::Reads(reader) if reader.expansions == Expansions::Included && matches!(reader.reads, Reads::Every)
    );
    syntax && child
}

fn the_native_semantic_readers_ignore_added_elements() -> bool {
    let syntax = matches!(
        native_semantic_syntax::<()>().as_ref(),
        Answer::Reads(reader) if reader.expansions == Expansions::Ignored && matches!(reader.reads, Reads::Every)
    );
    let child = matches!(
        native_semantic_child::<()>().as_ref(),
        Answer::Reads(reader) if reader.expansions == Expansions::Ignored && matches!(reader.reads, Reads::Every)
    );
    syntax && child
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
    Case {
        name: CaseName("the_expanded_semantic_readers_look_at_added_elements"),
        word: Word::Succeeds,
        input: || the_expanded_semantic_readers_look_at_added_elements,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_native_semantic_readers_ignore_added_elements"),
        word: Word::Succeeds,
        input: || the_native_semantic_readers_ignore_added_elements,
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
