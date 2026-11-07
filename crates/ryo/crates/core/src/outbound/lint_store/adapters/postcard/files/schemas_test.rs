//! What the schemas of the files of the store accept: a file reads back as it was written.

use super::{file_from, file_schema};
use crate::outbound::lint_store::adapters::postcard::strings::schemas::StringsSchema;
use crate::outbound::lint_store::fixtures::tables::stored;

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
        name: CaseName("a_file_reads_back_as_it_was_written"),
        word: Word::Succeeds,
        input: || (),
        expect: Expect::Value(true),
    },
];

/// Writes the files of a sample store through the table of strings and reads each back.
fn subject() -> Value {
    let files = stored(1).files;
    let mut strings = StringsSchema::default();
    let schemas: Vec<_> = files
        .iter()
        .map(|file| file_schema(&mut strings, file))
        .collect();
    schemas
        .iter()
        .zip(&files)
        .all(|(schema, file)| file_from(&strings.texts, schema).as_ref() == Ok(file))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| Ok(subject()));
}
