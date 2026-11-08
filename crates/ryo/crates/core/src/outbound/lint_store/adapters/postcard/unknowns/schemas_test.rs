//! What the schemas of the unknowns of the store accept: an unknown reads back as it was written, its cause with it.

use super::{unknown_from, unknown_schema};
use crate::outbound::lint_store::adapters::postcard::strings::schemas::StringsSchema;
use crate::outbound::lint_store::fixtures::tables::stored;
use ryo_values::domain::runs::types::UnknownCause;

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
        name: CaseName("an_unknown_reads_back_as_it_was_written_undecided_or_refused"),
        word: Word::Succeeds,
        input: || (),
        expect: Expect::Value(true),
    },
];

/// Writes the unknowns of a sample store, one undecided and one refused, through the table of strings and reads each back.
fn subject() -> Value {
    let unknowns = stored(1).unknowns;
    let mut strings = StringsSchema::default();
    let schemas: Vec<_> = unknowns
        .iter()
        .map(|unknown| unknown_schema(&mut strings, unknown))
        .collect();
    unknowns
        .iter()
        .any(|unknown| matches!(unknown.cause, UnknownCause::Refused(_)))
        && schemas
            .iter()
            .zip(&unknowns)
            .all(|(schema, unknown)| unknown_from(&strings.texts, schema).as_ref() == Ok(unknown))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| Ok(subject()));
}
