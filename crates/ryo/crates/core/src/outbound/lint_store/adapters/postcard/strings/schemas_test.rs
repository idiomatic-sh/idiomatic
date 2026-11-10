//! What the shared schemas of the store accept: a text is named by its place and written once.

use super::{StringsSchema, named, text};

use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases asks of the schemas.
type Input = ();
/// Whether the schemas answered as the row expects.
type Value = bool;
/// What a row of the cases expects when it is refused.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_text_is_named_by_its_place_and_written_once"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

/// Names one text twice and reads it back from its place.
fn subject() -> Value {
    let mut strings = StringsSchema::default();
    let first = text(&mut strings, "one");
    let again = text(&mut strings, "one");
    let other = text(&mut strings, "two");
    strings.texts.len() == 2
        && named(&strings.texts, first).as_deref().ok() == Some("one")
        && named(&strings.texts, again).as_deref().ok() == Some("one")
        && named(&strings.texts, other).as_deref().ok() == Some("two")
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| Ok(subject()));
}
