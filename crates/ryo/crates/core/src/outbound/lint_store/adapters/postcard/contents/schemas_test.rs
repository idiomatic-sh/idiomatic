//! What the schemas of the table of contents accept and refuse.

use super::{contents, contents_bytes};
use crate::outbound::lint_store::adapters::postcard::types::TableOfContents;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases asks of the schemas.
type Input = ();
/// Whether the schemas answered as the row expects.
type Value = bool;
/// What a row of the cases expects when it is refused.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("another_builds_table_of_contents_reads_as_nothing"),
        word: Word::Succeeds,
        input: || (),
        expect: Expect::Value(true),
    },
];

/// Writes the table of contents for one build and reads it back by that build and by another.
fn subject() -> Value {
    let bytes = contents_bytes("b", &TableOfContents::default()).expect("written");
    contents("b", &bytes).is_some() && contents("c", &bytes).is_none()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| Ok(subject()));
}
