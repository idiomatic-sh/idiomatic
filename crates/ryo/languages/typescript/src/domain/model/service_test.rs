//! The `model` service's cases: clearing the indexes of one file leaves nothing of it for the next.

use super::service::clear;
use super::types::ReadIndexes;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::bindings::service::indexed;
use crate::domain::shapes::service::prepare;
use crate::domain::syntax::types::Dialect;
use crate::outbound::syntax::{factory, interface::SyntaxPort as _, types::SourceText};
use ryo::languages;

/// Which index a row builds before clearing the model.
#[derive(Clone, Copy)]
enum Index {
    /// The names the file's declarations bind.
    Bindings,
    /// The structural classes of the file's nodes.
    Shapes,
}

/// The index a row builds, and the source the last file held.
type Input = (Index, &'static str);
/// What the index held before and after clearing, written out.
type Value = &'static str;
/// Clearing is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("clearing_empties_the_names_of_the_last_file_for_the_next"),
        word: Word::Succeeds,
        input: || {
            (
                Index::Bindings,
                "let a = 1; function f(b) { return a + b; }",
            )
        },
        expect: Expect::Value("3 names, then 0 names Pending"),
    },
    Case {
        name: CaseName("clearing_forgets_the_last_file_and_rebuilds_on_demand"),
        word: Word::Succeeds,
        input: || (Index::Shapes, "a + b;\n"),
        expect: Expect::Value("Some(12)|Complete|0|Fresh|Some(12)"),
    },
];

/// Builds the row's index over its source, clears the model, and says what the index held at each step.
fn subject((index, source): Input) -> Result<Value, Failure> {
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
    let written = match index {
        Index::Bindings => {
            let before = indexed(&tree, &indexes.bindings).names.len();
            clear(&mut indexes);
            let after = indexes.bindings.borrow();
            format!(
                "{before} names, then {} names {:?}",
                after.names.len(),
                after.state
            )
        }
        Index::Shapes => {
            let built = prepare(&tree, &indexes, &mut symbols).map(|shapes| shapes.classes.len());
            let state = indexes.shapes.borrow().state;
            clear(&mut indexes);
            let cleared = indexes.shapes.borrow().classes.len();
            let after = indexes.shapes.borrow().state;
            let rebuilt = prepare(&tree, &indexes, &mut symbols).map(|shapes| shapes.classes.len());
            format!("{built:?}|{state:?}|{cleared}|{after:?}|{rebuilt:?}")
        }
    };
    Ok(Box::leak(written.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
