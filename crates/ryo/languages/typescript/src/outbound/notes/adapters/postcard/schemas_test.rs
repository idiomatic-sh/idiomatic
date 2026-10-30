//! The schemas' cases: what is converted into the wire shapes comes back as it was.

use super::{ImportSchema, ImportViewSchema, NoteSchema, NoteViewSchema, OffsetSchema};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::types::{ImportNote, ImportSpelling, Note, Specifier};
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
            text: ImportSpelling("import'./ab'".into()),
        }],
    }
}

fn note_is_read_as_it_is_written() -> String {
    let schema = NoteSchema {
        imports: vec![ImportSchema {
            specifier: "./a b".to_owned(),
            start: OffsetSchema(3),
            end: OffsetSchema(9),
            text: "import'./ab'".to_owned(),
        }],
    };
    (Note::from(schema) == note()).to_string()
}

fn note_is_written_as_a_view_of_itself() -> String {
    let note = note();
    let expected = NoteViewSchema {
        imports: vec![ImportViewSchema {
            specifier: "./a b",
            start: OffsetSchema(3),
            end: OffsetSchema(9),
            text: "import'./ab'",
        }],
    };
    (NoteViewSchema::from(&note) == expected).to_string()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_note_is_read_from_its_wire_shape_as_it_was_written"),
        word: Word::Succeeds,
        input: || note_is_read_as_it_is_written,
        expect: Expect::Value("true"),
    },
    Case {
        name: CaseName("a_note_is_written_as_a_view_that_borrows_its_specifiers"),
        word: Word::Succeeds,
        input: || note_is_written_as_a_view_of_itself,
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
