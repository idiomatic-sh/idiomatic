//! The `fact_notes` service's cases, on values alone: the bytes the notes are written as are the port's.

use super::service::{facts_of, note_of};
use super::types::Naming;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::module_tree::types::{
    Body, Cleanliness, EnclosingBlock, Facts, ModDeclaration, Segment, Spelling, Written,
};
use crate::domain::paths::types::Section;
use crate::domain::targets::types::WrittenPath;
use ryo::languages::Offset;

/// A scenario: what is asked of the notes of the facts, shown.
type Input = fn() -> String;
/// What it came to.
type Value = &'static str;
type Failure = ();

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
        bound: None,
    }
}

fn read_back() -> String {
    (facts_of(note_of(&facts()), &|_| Naming::Known) == facts()).to_string()
}

fn unnamed_paths() -> String {
    let kept = facts_of(note_of(&facts()), &|first| {
        if &**first == "leaf" {
            Naming::Known
        } else {
            Naming::Unknown
        }
    });
    let first: Vec<&str> = kept.written.iter().map(|each| &*each.segments[0]).collect();
    format!("{} {}", first.join(" "), kept.declared == facts().declared)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("facts_are_read_back_from_their_notes"),
        word: Word::Succeeds,
        input: || read_back,
        expect: Expect::Value("true"),
    },
    Case {
        name: CaseName("a_path_that_names_nothing_of_the_repository_is_left_out"),
        word: Word::Succeeds,
        input: || unnamed_paths,
        expect: Expect::Value("crate leaf true"),
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
