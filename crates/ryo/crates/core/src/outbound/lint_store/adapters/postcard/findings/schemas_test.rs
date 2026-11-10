//! What the schemas of the findings of the store accept: a cell reads back as it was written.

use super::{cell_from, cell_schema};
use crate::outbound::lint_store::adapters::postcard::strings::schemas::StringsSchema;
use ryo_values::domain::row_texts::types::Cell;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases asks of the schemas.
type Input = ();
/// Whether the schemas answered as the row expects.
type Value = bool;
/// What a row of the cases expects when it is refused.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_cell_reads_back_as_it_was_written"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

/// Writes a text cell and a file cell through the table of strings and reads each back.
fn subject() -> Value {
    let cells = [
        Cell::File("src/a.rs".into()),
        Cell::Number(ryo_relations::domain::relations::types::Integer(7)),
    ];
    let mut strings = StringsSchema::default();
    let schemas: Vec<_> = cells
        .iter()
        .map(|cell| cell_schema(&mut strings, cell))
        .collect();
    schemas
        .into_iter()
        .zip(&cells)
        .all(|(schema, cell)| cell_from(&strings.texts, schema).as_ref() == Ok(cell))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| Ok(subject()));
}
