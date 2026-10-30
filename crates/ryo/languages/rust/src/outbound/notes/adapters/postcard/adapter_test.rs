//! The `postcard` adapter of the notes port: the port's contract rows run through it, then the cases specific to it: the bytes postcard
//! writes, and what is nothing.

use super::adapter::NotesPostcardAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::fact_notes::service::{facts_of, note_of};
use crate::domain::fact_notes::types::Naming;
use crate::domain::module_tree::types::{
    Body, Cleanliness, EnclosingBlock, Facts, ModDeclaration, Segment, Spelling, Written,
};
use crate::domain::paths::types::Section;
use crate::domain::targets::types::WrittenPath;
use crate::outbound::notes::errors::NotesError;
use crate::outbound::notes::fixtures::contract::check;
use crate::outbound::notes::interface::NotesPort as _;
use ryo::languages::{Bytes, Offset};

/// A scenario: what is done with the notes, shown.
type Input = fn() -> Result<String, NotesError>;
/// What it came to.
type Value = &'static str;
type Failure = NotesError;

fn facts() -> Facts {
    Facts {
        clean: Cleanliness::Clean,
        declared: vec![
            ModDeclaration {
                around: vec![EnclosingBlock {
                    name: Segment("outer".into()),
                    path: Section::Present(WrittenPath("x".into())),
                }],
                name: Segment("leaf".into()),
                body: Body::File,
                path: Section::Absent,
            },
            ModDeclaration {
                around: Vec::new(),
                name: Segment("inner".into()),
                body: Body::Inline,
                path: Section::Present(WrittenPath("y.rs".into())),
            },
        ],
        written: vec![
            written(&["crate", "a"], &["outer"], (3, 9)),
            written(&["serde", "Serialize"], &[], (11, 300_000)),
            written(&["leaf", "f"], &[], (31, 38)),
        ],
    }
}

fn written(segments: &[&str], enclosing: &[&str], place: (u32, u32)) -> Written {
    Written {
        segments: segments
            .iter()
            .map(|each| Segment((*each).into()))
            .collect(),
        enclosing: enclosing
            .iter()
            .map(|each| Segment((*each).into()))
            .collect(),
        start: Offset(place.0),
        end: Offset(place.1),
        text: Spelling(segments.join("::").into()),
    }
}

fn written_bytes(facts: &Facts) -> Bytes {
    let mut out = Bytes::default();
    NotesPostcardAdapter.write(&note_of(facts), &mut out);
    out
}

fn read_back() -> Result<String, NotesError> {
    let bytes = written_bytes(&facts());
    let note = NotesPostcardAdapter.note_in(&bytes);
    note.map(|each| (facts_of(each, &|_| Naming::Known) == facts()).to_string())
}

fn cut_short() -> Result<String, NotesError> {
    let bytes = written_bytes(&facts());
    NotesPostcardAdapter
        .note_in(&Bytes(bytes.0[..bytes.0.len() - 1].to_vec()))
        .map(|_| String::new())
}

fn trailing() -> Result<String, NotesError> {
    let mut bytes = written_bytes(&facts());
    bytes.0.push(0);
    NotesPostcardAdapter.note_in(&bytes).map(|_| String::new())
}

fn tiny_bytes() -> String {
    let tiny = Facts {
        clean: Cleanliness::Clean,
        declared: vec![ModDeclaration {
            around: Vec::new(),
            name: Segment("a".into()),
            body: Body::File,
            path: Section::Absent,
        }],
        written: Vec::new(),
    };
    format!("{:?}", written_bytes(&tiny).0)
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
        expect: Expect::Refusal(|error| selects(matches!(error, NotesError::NotANote))),
    },
    Case {
        name: CaseName("a_note_is_written_as_postcard_writes_it"),
        word: Word::Succeeds,
        input: || || Ok(tiny_bytes()),
        expect: Expect::Value("[1, 1, 0, 1, 97, 0, 0, 0]"),
    },
    Case {
        name: CaseName("bytes_followed_by_more_are_no_note"),
        word: Word::Refuses,
        input: || trailing,
        expect: Expect::Refusal(|error| selects(matches!(error, NotesError::NotANote))),
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
