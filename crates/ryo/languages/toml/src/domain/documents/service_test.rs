//! The `documents` service's cases: the mappings, entries and sequence elements the one pass over a file's tree works out. The model's
//! rows are held in full by the items' cases.

use super::service::build;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::types::Model;
use crate::fixtures::contracts::parse;
use ryo::languages;

/// A file.
type Input = &'static str;
/// How many mappings, entries and sequence elements the pass works out of it.
type Value = (usize, usize, usize);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_key_value_is_an_entry_of_the_root_mapping"),
        word: Word::Succeeds,
        input: || "a = 1\n",
        expect: Expect::Value((1, 1, 0)),
    },
    Case {
        name: CaseName("a_header_opens_a_mapping_and_is_an_entry_of_the_root"),
        word: Word::Succeeds,
        input: || "[t]\nb = 2\n",
        expect: Expect::Value((2, 2, 0)),
    },
    Case {
        name: CaseName("each_value_of_an_array_is_a_sequence_element"),
        word: Word::Succeeds,
        input: || "a = [1, 2]\n",
        expect: Expect::Value((1, 1, 2)),
    },
    Case {
        name: CaseName("an_inline_table_is_a_mapping_of_its_own"),
        word: Word::Succeeds,
        input: || "a = {b = 1}\n",
        expect: Expect::Value((2, 2, 0)),
    },
];

fn subject(text: Input) -> Result<Value, Failure> {
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let request = languages::ParseRequest {
        path: languages::SourcePath("documents.toml"),
        file: languages::FileId(0),
        bytes: languages::ByteSlice(text.as_bytes()),
        notes: languages::NoteRequest::Unwanted,
        fields: languages::FieldRequest::Unwanted,
    };
    languages::contract::build(&parse(), request, &mut elements, &mut symbols).map_err(drop)?;
    let mut model = Model::default();
    build(
        &languages::Tree {
            elements: &elements,
            source: languages::ByteSlice(text.as_bytes()),
            file: languages::FileId(0),
        },
        &mut model,
    );
    Ok((
        model.mappings.len(),
        model.entries.len(),
        model.elements.len(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
