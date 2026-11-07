//! The `fact_notes` service's cases: the imports a file writes are noted alias by alias, each with its form, its dots and its place.

use super::service::noted;
use super::types::Note;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

/// The source of a file: its notes, each import shown as `form level path start..end`.
type Input = &'static str;
type Value = &'static [&'static str];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_alias_is_noted_with_its_form_its_dots_and_its_place"),
        word: Word::Succeeds,
        input: || "import a.b\nimport a.b as c\nfrom . import x\nfrom ..m import y as z\nfrom m import *\nfrom ... import *\n",
        expect: Expect::Value(&[
            "Module 0 a.b 7..10",
            "Module 0 a.b 18..26",
            "Name 1 x 41..42",
            "Name 2 m.y 59..65",
            "Everything 0 m 80..81",
            "Everything 3  98..99",
        ]),
    },
    Case {
        name: CaseName("a_file_with_no_import_is_noted_with_none"),
        word: Word::Succeeds,
        input: || "x = 1\n",
        expect: Expect::Value(&[]),
    },
];

/// Each import of `note`, as `form level path start..end`.
fn shown(note: &Note) -> Vec<String> {
    note.imports
        .iter()
        .map(|import| {
            format!(
                "{:?} {} {} {}..{}",
                import.form, import.level.0, import.path.0, import.start.0, import.end.0
            )
        })
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

fn subject(source: Input) -> Value {
    let file = parsed(source.as_bytes()).expect("the row's source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    leaked(shown(&noted(&tree)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
