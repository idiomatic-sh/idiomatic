//! The `models` service's cases: a reader is answered from the model of its own file.

use super::service::entry_at;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::documents::service::build;
use crate::fixtures::contracts::parse;
use ryo::languages::{self, Position};

/// A file, and the element asked about.
type Input = (&'static str, u32);
/// The owner and the ordinal of the entry at that element.
type Value = Option<(u32, i64)>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_entry_is_answered_with_its_owner_and_ordinal"),
        word: Word::Succeeds,
        input: || ("[t]\nb = 2\n", 9),
        expect: Expect::Value(Some((1, 0))),
    },
    Case {
        name: CaseName("an_element_that_is_no_entry_has_none"),
        word: Word::Succeeds,
        input: || ("a = 1\n", 0),
        expect: Expect::Value(None),
    },
];

fn subject((text, at): Input) -> Result<Value, Failure> {
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let request = languages::ParseRequest {
        path: languages::SourcePath("model.toml"),
        file: languages::FileId(0),
        bytes: languages::ByteSlice(text.as_bytes()),
        notes: languages::NoteRequest::Unwanted,
        fields: languages::FieldRequest::Unwanted,
    };
    languages::contract::build(&parse(), request, &mut elements, &mut symbols).map_err(drop)?;
    let mut model = crate::domain::model::types::Model::default();
    build(
        &languages::Tree {
            elements: &elements,
            source: languages::ByteSlice(text.as_bytes()),
            file: languages::FileId(0),
        },
        &mut model,
    );
    Ok(entry_at(&model, Position(at)).map(|fact| (fact.owner.0, fact.ordinal.0)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
