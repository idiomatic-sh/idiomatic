//! The schemas' cases: what is converted into the wire shapes comes back as it was.

use super::{
    FormSchema, ImportSchema, LevelSchema, NoteSchema, NoteViewSchema, OffsetSchema, TextSchema,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::types::{DottedPath, Form, ImportNote, Level, Note, Spelling};
use ryo::languages::Offset;

/// A scenario: what is converted, and shown.
type Input = fn() -> String;
/// What it came to.
type Value = &'static str;
type Failure = ();

fn note() -> Note {
    Note {
        imports: vec![ImportNote {
            form: Form::Name,
            level: Level(2),
            path: DottedPath("a.b".into()),
            start: Offset(3),
            end: Offset(9),
            text: Spelling("b".into()),
        }],
    }
}

fn note_is_read_as_it_is_written() -> String {
    let schema = NoteSchema {
        imports: vec![ImportSchema {
            form: FormSchema::Name,
            level: LevelSchema(2),
            path: TextSchema("a.b".into()),
            start: OffsetSchema(3),
            end: OffsetSchema(9),
            text: TextSchema("b".into()),
        }],
    };
    (Note::from(schema) == note()).to_string()
}

fn view_holds_the_note() -> String {
    let note = note();
    let view = NoteViewSchema::from(&note);
    format!("{view:?}")
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
        name: CaseName("a_note_is_written_from_a_view_of_it"),
        word: Word::Succeeds,
        input: || view_holds_the_note,
        expect: Expect::Value(
            "NoteViewSchema { imports: [ImportViewSchema { form: Name, level: LevelSchema(2), path: \"a.b\", start: OffsetSchema(3), end: OffsetSchema(9), text: \"b\" }] }",
        ),
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
