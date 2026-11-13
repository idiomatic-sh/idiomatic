//! The `model` service's cases: clearing the model for the next file leaves no class of the last one.

use super::service::clear;
use super::types::ReadIndexes;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::shapes::service::prepare;
use crate::domain::syntax::types::Dialect;
use crate::outbound::syntax::{factory, interface::SyntaxPort as _, types::SourceText};
use ryo::languages;

/// The source the last file held.
type Input = &'static str;
/// What the shape index held before and after clearing, written out.
type Value = &'static str;
/// Clearing is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("clearing_forgets_the_last_file_and_rebuilds_on_demand"),
    word: Word::Succeeds,
    input: || "a + b;\n",
    expect: Expect::Value("Some(12)|Complete|0|Fresh|Some(12)"),
}];

/// Builds the shapes of the row's source, clears the model, and says what the index held at each step.
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
    let mut indexes = ReadIndexes::default();
    let built = prepare(&tree, &indexes, &mut symbols).map(|index| index.classes.len());
    let state = indexes.shapes.borrow().state;
    clear(&mut indexes);
    let cleared = indexes.shapes.borrow().classes.len();
    let after = indexes.shapes.borrow().state;
    let rebuilt = prepare(&tree, &indexes, &mut symbols).map(|index| index.classes.len());
    Ok(Box::leak(
        format!("{built:?}|{state:?}|{cleared}|{after:?}|{rebuilt:?}").into_boxed_str(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
