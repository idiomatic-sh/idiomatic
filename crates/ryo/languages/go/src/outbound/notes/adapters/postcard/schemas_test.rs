//! The schemas' cases: what is converted into the wire shapes comes back as it was.

use super::{ImportSchema, NoteSchema, OffsetSchema, TextSchema};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::types::{ImportNote, ImportPath, Note, Spelling};
use ryo::languages::Offset;

/// A scenario: what is converted, and shown.
type Input = fn() -> String;
/// What it came to.
type Value = &'static str;
type Failure = ();

fn note_is_read_as_it_is_written() -> String {
    let schema = NoteSchema {
        imports: vec![ImportSchema {
            path: TextSchema("example.com/a b".into()),
            start: OffsetSchema(3),
            end: OffsetSchema(9),
            text: TextSchema("\"example.com/a\"".into()),
        }],
    };
    let expected = Note {
        imports: vec![ImportNote {
            path: ImportPath("example.com/a b".into()),
            start: Offset(3),
            end: Offset(9),
            text: Spelling("\"example.com/a\"".into()),
        }],
    };
    (Note::from(schema) == expected).to_string()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_note_is_read_from_its_wire_shape_as_it_was_written"),
    word: Word::Succeeds,
    input: || note_is_read_as_it_is_written,
    expect: Expect::Value("true"),
}];

fn subject(scenario: Input) -> Value {
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(scenario().into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
