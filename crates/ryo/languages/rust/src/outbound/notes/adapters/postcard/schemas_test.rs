//! The schemas' cases: what is converted into the wire shapes comes back as it was.

use super::{
    DeclaredSchema, FlagSchema, NameSchema, NoteSchema, OffsetSchema, StepSchema, TextSchema,
    WrittenSchema,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::types::{
    DeclaredNote, Note, NoteFlag, NoteName, NoteOffset, NotePath, StepNote, WrittenNote,
};
use std::borrow::Cow;

/// A scenario: what is converted, and shown.
type Input = fn() -> String;
/// What it came to.
type Value = &'static str;
type Failure = ();

fn note_is_read_as_it_is_written() -> String {
    let schema = NoteSchema {
        clean: FlagSchema(true),
        declared: vec![DeclaredSchema {
            test_only: FlagSchema(false),
            exported: FlagSchema(true),
            around: vec![StepSchema {
                name: NameSchema("outer"),
                path: Some(TextSchema("x".into())),
            }],
            name: NameSchema("leaf"),
            inline: FlagSchema(false),
            path: None,
        }],
        written: vec![WrittenSchema {
            segments: vec![NameSchema("crate"), NameSchema("a")],
            enclosing: Vec::new(),
            start: OffsetSchema(3),
            end: OffsetSchema(9),
            text: NameSchema("crate::a"),
            bound: None,
            scope: None,
        }],
    };
    let expected = Note {
        clean: NoteFlag(true),
        declared: vec![DeclaredNote {
            test_only: NoteFlag(false),
            exported: NoteFlag(true),
            around: vec![StepNote {
                name: NoteName(Cow::Borrowed("outer")),
                path: Some(NotePath("x".into())),
            }],
            name: NoteName(Cow::Borrowed("leaf")),
            inline: NoteFlag(false),
            path: None,
        }],
        written: vec![WrittenNote {
            segments: vec![
                NoteName(Cow::Borrowed("crate")),
                NoteName(Cow::Borrowed("a")),
            ],
            enclosing: Vec::new(),
            start: NoteOffset(3),
            end: NoteOffset(9),
            text: NoteName(Cow::Borrowed("crate::a")),
            bound: None,
            scope: None,
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
