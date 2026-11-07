//! The `postcard` adapter of the notes port: the port's contract rows run through it, then the cases specific to it: the bytes postcard
//! writes, and what is nothing.

use super::adapter::NotesPostcardAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;
use crate::domain::fact_notes::service::noted;
use crate::domain::fact_notes::types::{DottedPath, Form, ImportNote, Level, Note, Spelling};
use crate::fixtures::reading::parsed;
use crate::outbound::notes::errors::NotesError;
use crate::outbound::notes::fixtures::contract::check;
use crate::outbound::notes::interface::NotesPort as _;
use ryo::languages::{self, ByteSlice, Bytes, Offset, Tree};

/// A scenario: what is done with the notes, shown.
type Input = fn() -> Result<String, NotesError>;
/// What it came to.
type Value = &'static str;
type Failure = NotesError;

fn one_import() -> Note {
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

fn written_bytes(note: &Note) -> Bytes {
    let mut out = Bytes(vec![9, 9, 9]);
    NotesPostcardAdapter.write(note, &mut out);
    out
}

fn read_back() -> Result<String, NotesError> {
    let bytes = written_bytes(&one_import());
    NotesPostcardAdapter
        .note_in(&bytes)
        .map(|note| (note == one_import()).to_string())
}

fn source_read_back() -> Result<String, NotesError> {
    let source = "import a.b\nfrom ..m import y as z\nfrom m import *\n";
    let file = parsed(source.as_bytes()).expect("the row's source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let note = noted(&tree);
    NotesPostcardAdapter
        .note_in(&written_bytes(&note))
        .map(|each| (each == note && note.imports.len() == 3).to_string())
}

fn cut_short() -> Result<String, NotesError> {
    let bytes = written_bytes(&one_import());
    NotesPostcardAdapter
        .note_in(&Bytes(bytes.0[..bytes.0.len() - 1].to_vec()))
        .map(|_| String::new())
}

fn trailing() -> Result<String, NotesError> {
    let mut bytes = written_bytes(&one_import());
    bytes.0.push(0);
    NotesPostcardAdapter.note_in(&bytes).map(|_| String::new())
}

fn no_form() -> Result<String, NotesError> {
    NotesPostcardAdapter
        .note_in(&Bytes(vec![1, 3, 0, 0, 0, 0]))
        .map(|_| String::new())
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
        name: CaseName("the_notes_of_a_file_are_read_back_as_the_parse_noted_them"),
        word: Word::Succeeds,
        input: || source_read_back,
        expect: Expect::Value("true"),
    },
    Case {
        name: CaseName("a_note_is_written_as_postcard_writes_it"),
        word: Word::Succeeds,
        input: || || Ok(format!("{:?}", written_bytes(&one_import()).0)),
        expect: Expect::Value("[1, 1, 2, 3, 97, 46, 98, 3, 9, 1, 98]"),
    },
    Case {
        name: CaseName("bytes_cut_short_are_no_note"),
        word: Word::Refuses,
        input: || cut_short,
        expect: Expect::Refusal(|error| selects!(error, NotesError::NotANote)),
    },
    Case {
        name: CaseName("bytes_followed_by_more_are_no_note"),
        word: Word::Refuses,
        input: || trailing,
        expect: Expect::Refusal(|error| selects!(error, NotesError::NotANote)),
    },
    Case {
        name: CaseName("an_import_of_no_form_is_no_note"),
        word: Word::Refuses,
        input: || no_form,
        expect: Expect::Refusal(|error| selects!(error, NotesError::NotANote)),
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
