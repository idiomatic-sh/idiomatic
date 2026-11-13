//! The `model` service's cases: clearing the indexes of one file before the next.

use super::service::clear;
use super::types::ReadIndexes;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::bindings::service::indexed;
use crate::domain::syntax::types::Dialect;
use crate::outbound::syntax::{factory, interface::SyntaxPort as _, types::SourceText};
use ryo::languages;

/// The source whose names a row indexes before clearing them.
type Input = &'static str;
/// How many names the index held before the clear, and what it holds after it.
type Value = &'static str;
/// Clearing is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("clearing_empties_the_names_of_the_last_file_for_the_next"),
    word: Word::Succeeds,
    input: || "let a = 1; function f(b) { return a + b; }",
    expect: Expect::Value("3 names, then 0 names Pending"),
}];

/// Indexes the row's source, clears the model, and writes out the index before and after.
fn subject(source: Input) -> Result<Value, Failure> {
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let bytes = languages::ByteSlice(source.as_bytes());
    factory::oxc_parser().read(
        SourceText(source.as_bytes()),
        Dialect::TypeScript,
        &mut languages::table_builder(&mut elements, &mut symbols, bytes),
    );
    let tree = languages::Tree {
        elements: &elements,
        source: bytes,
        file: languages::FileId(0),
    };
    let mut model = ReadIndexes::default();
    let before = indexed(&tree, &model.bindings).names.len();
    clear(&mut model);
    let after = model.bindings.borrow();
    Ok(Box::leak(
        format!(
            "{before} names, then {} names {:?}",
            after.names.len(),
            after.state
        )
        .into_boxed_str(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
