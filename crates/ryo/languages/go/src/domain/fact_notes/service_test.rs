//! The `fact_notes` service's cases: each package a file imports is noted with the place of its spec. The bytes the notes are written as
//! are the port's.

use super::service::noted;
use super::types::Note;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::asked;

/// The text of a file: its notes, each import shown as `path start..end`.
type Input = &'static str;
type Value = &'static [&'static str];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_spec_of_an_import_is_noted_with_its_place"),
        word: Word::Succeeds,
        input: || "package item\n\nimport \"fmt\"\n\nimport (\n\tstore \"example.com/app/store\"\n\t_ `example.com/app/raw`\n\t. \"example.com/app/dot\"\n)\n",
        expect: Expect::Value(&[
            "fmt 21..26",
            "example.com/app/store 38..67",
            "example.com/app/raw 69..92",
            "example.com/app/dot 94..117",
        ]),
    },
    Case {
        name: CaseName("a_file_with_no_import_is_noted_with_none"),
        word: Word::Succeeds,
        input: || "package item\n",
        expect: Expect::Value(&[]),
    },
];

/// Each import of `note`, as `path start..end`.
fn shown(note: &Note) -> Vec<String> {
    note.imports
        .iter()
        .map(|import| format!("{} {}..{}", import.path.0, import.start.0, import.end.0))
        .collect()
}

/// `lines`, leaked: a case compares them with text that lives for the whole run.
fn leaked(lines: Vec<String>) -> Value {
    let lines: Vec<&'static str> = lines
        .into_iter()
        .map(|line| &*Box::leak(line.into_boxed_str()))
        .collect();
    Box::leak(lines.into_boxed_slice())
}

fn subject(text: Input) -> Value {
    leaked(shown(&asked(text, noted)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<_, Failure>(subject(input))
    });
}
