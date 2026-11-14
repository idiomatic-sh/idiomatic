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
/// How many nodes the shapes classed before and after clearing, and whether clearing emptied the index, written out.
type Value = &'static str;
/// Clearing is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("clearing_forgets_the_last_file_and_rebuilds_on_demand"),
    word: Word::Succeeds,
    input: || "fn f() { a + b; }\n",
    expect: Expect::Value("Some(16)|true|Some(16)"),
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
    let classed = |shapes: languages::ShapeClasses<'_>| {
        languages::positions(tree.elements)
            .filter_map(|at| languages::shape_class_at(&shapes, at))
            .count()
    };
    let built = prepare(&tree, &indexes, &mut symbols).map(classed);
    clear(&mut indexes);
    let cleared = indexes.shapes == languages::ShapeIndex::default();
    let rebuilt = prepare(&tree, &indexes, &mut symbols).map(classed);
    Ok(Box::leak(
        format!("{built:?}|{cleared}|{rebuilt:?}").into_boxed_str(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
