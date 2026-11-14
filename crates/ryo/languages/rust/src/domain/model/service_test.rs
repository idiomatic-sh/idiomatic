//! The `model` service's cases: clearing the indexes of one file leaves nothing of it for the next.

use super::service::clear;
use super::types::ReadIndexes;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::shapes::service::prepare;
use crate::outbound::syntax::{
    factory,
    interface::SyntaxPort as _,
    types::{Expanding, SourceText},
};
use ryo::languages;

/// The source the last file held.
type Input = &'static str;
/// What the shapes index held before and after clearing, written out.
type Value = &'static str;
/// Clearing is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("clearing_forgets_the_last_file_and_rebuilds_on_demand"),
    word: Word::Succeeds,
    input: || "fn f() { a + b; }\n",
    expect: Expect::Value("Some(33)|Complete|0|Fresh|Some(33)"),
}];

/// Builds the shapes over the row's source, clears the model, and says what the index held at each step.
fn subject(source: Input) -> Result<Value, Failure> {
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let bytes = languages::ByteSlice(source.as_bytes());
    factory::ra_ap_syntax(Expanding(&[])).read(
        SourceText(source.as_bytes()),
        &mut languages::table_builder(&mut elements, &mut symbols, bytes),
    );
    let tree = languages::Tree {
        elements: &elements,
        source: bytes,
        file: languages::FileId(0),
    };
    let mut indexes = ReadIndexes::default();
    let built = prepare(&tree, &indexes, &mut symbols).map(|shapes| shapes.classes.len());
    let state = indexes.shapes.borrow().state;
    clear(&mut indexes);
    let cleared = indexes.shapes.borrow().classes.len();
    let after = indexes.shapes.borrow().state;
    let rebuilt = prepare(&tree, &indexes, &mut symbols).map(|shapes| shapes.classes.len());
    Ok(Box::leak(
        format!("{built:?}|{state:?}|{cleared}|{after:?}|{rebuilt:?}").into_boxed_str(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
