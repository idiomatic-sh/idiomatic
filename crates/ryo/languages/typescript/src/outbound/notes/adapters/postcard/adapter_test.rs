//! The `postcard` adapter of the notes port: the port's contract rows run through it, then the cases specific to it: the bytes postcard
//! writes, and what is nothing.

use super::adapter::NotesPostcardAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::types::{ImportNote, ImportSpelling, Note, Specifier};
use crate::outbound::notes::errors::NotesError;
use crate::outbound::notes::fixtures::contract::check;
use crate::outbound::notes::interface::NotesPort as _;
use ryo::languages::{Bytes, Offset};

/// A scenario: what is done with the notes, shown.
type Input = fn() -> Result<String, NotesError>;
/// What it came to.
type Value = &'static str;
type Failure = NotesError;

fn note() -> Note {
    Note {
        imports: vec![
            ImportNote {
                specifier: Specifier("./a b".into()),
                start: Offset(3),
                end: Offset(9),
                text: ImportSpelling("import'./ab'".into()),
            },
            ImportNote {
                specifier: Specifier("@scope/pkg".into()),
                start: Offset(11),
                end: Offset(300_000),
                text: ImportSpelling("import'@scope/pkg'".into()),
            },
        ],
    }
}

fn written_bytes(note: &Note) -> Bytes {
    let mut out = Bytes(vec![9, 9, 9]);
    NotesPostcardAdapter.write(note, &mut out);
    out
}

fn read_back() -> Result<String, NotesError> {
    let bytes = written_bytes(&note());
    NotesPostcardAdapter
        .note_in(&bytes)
        .map(|each| (each == note()).to_string())
}

fn cut_short() -> Result<String, NotesError> {
    let bytes = written_bytes(&note());
    NotesPostcardAdapter
        .note_in(&Bytes(bytes.0[..bytes.0.len() - 1].to_vec()))
        .map(|_| String::new())
}

fn trailing() -> Result<String, NotesError> {
    let mut bytes = written_bytes(&note());
    bytes.0.push(0);
    NotesPostcardAdapter.note_in(&bytes).map(|_| String::new())
}

fn tiny_bytes() -> String {
    let tiny = Note {
        imports: vec![ImportNote {
            specifier: Specifier("a".into()),
            start: Offset(1),
            end: Offset(2),
            text: ImportSpelling("a".into()),
        }],
    };
    format!("{:?}", written_bytes(&tiny).0)
}

fn no_import_bytes() -> String {
    format!("{:?}", written_bytes(&Note::default()).0)
}

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("notes_are_read_back_as_they_were_written"),
        word: Word::Succeeds,
        input: || read_back,
        expect: Expect::Value("true"),
    },
    Case {
        name: CaseName("bytes_cut_short_are_no_note"),
        word: Word::Refuses,
        input: || cut_short,
        expect: Expect::Error(NotesError::NotANote),
    },
    Case {
        name: CaseName("a_note_is_written_as_postcard_writes_it"),
        word: Word::Succeeds,
        input: || || Ok(tiny_bytes()),
        expect: Expect::Value("[1, 1, 97, 1, 2, 1, 97]"),
    },
    Case {
        name: CaseName("a_note_of_no_import_is_an_empty_list"),
        word: Word::Succeeds,
        input: || || Ok(no_import_bytes()),
        expect: Expect::Value("[0]"),
    },
    Case {
        name: CaseName("bytes_followed_by_more_are_no_note"),
        word: Word::Refuses,
        input: || trailing,
        expect: Expect::Error(NotesError::NotANote),
    },
];

fn subject(shown: String) -> Value {
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(shown.into_boxed_str())
}

#[test]
fn succeeds() {
    check(&NotesPostcardAdapter);
    run(CASES, Word::Succeeds, |input| input().map(subject));
    run(CASES, Word::Refuses, |input| input().map(subject));
}
