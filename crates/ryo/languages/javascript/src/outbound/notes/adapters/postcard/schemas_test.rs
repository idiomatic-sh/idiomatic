//! The schemas' cases: what is converted into the wire shape comes back as it was.

use super::{ImportSchema, NoteSchema, OffsetSchema};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::types::{ImportNote, Note, Specifier, Spelling};
use ryo::languages::Offset;

/// A scenario: what is converted, and shown.
type Input = fn() -> String;
/// What it came to.
type Value = &'static str;
type Failure = ();

fn note() -> Note {
    Note {
        imports: vec![ImportNote {
            specifier: Specifier("./a b".into()),
            start: Offset(3),
            end: Offset(9),
            text: Spelling("import'./ab'".into()),
        }],
    }
}

fn note_is_written_in_its_wire_shape() -> String {
    let expected = NoteSchema {
        imports: vec![ImportSchema {
            specifier: "./a b",
            start: OffsetSchema(3),
            end: OffsetSchema(9),
            text: "import'./ab'",
        }],
    };
    let note = note();
    (NoteSchema::from(&note) == expected).to_string()
}

fn note_is_read_from_its_wire_shape() -> String {
    let schema = NoteSchema {
        imports: vec![ImportSchema {
            specifier: "./a b",
            start: OffsetSchema(3),
            end: OffsetSchema(9),
            text: "import'./ab'",
        }],
    };
    (Note::from(schema) == note()).to_string()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_note_is_written_in_its_wire_shape"),
        word: Word::Succeeds,
        input: || note_is_written_in_its_wire_shape,
        expect: Expect::Value("true"),
    },
    Case {
        name: CaseName("a_note_is_read_from_its_wire_shape_as_it_was_written"),
        word: Word::Succeeds,
        input: || note_is_read_from_its_wire_shape,
        expect: Expect::Value("true"),
    },
];

fn subject(scenario: Input) -> Value {
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(scenario().into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
