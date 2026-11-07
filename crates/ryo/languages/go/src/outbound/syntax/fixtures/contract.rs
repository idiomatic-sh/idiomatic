//! What every `syntax` adapter must pass.

use crate::outbound::syntax::types::SourceText;
use ryo::languages::{ByteSlice, Elements, Symbols, end_of, positions, table_builder};

use super::super::interface::SyntaxPort;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract is given: go text.
type Input = &'static str;
/// Whether the table is a tree: one root, and every element's subtree inside its parent's.
type Value = bool;
/// A syntax port refuses nothing: the parser recovers from what it cannot read.
type Failure = ();

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_line_gives_a_well_formed_tree"),
        word: Word::Succeeds,
        input: || "a\n",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_empty_file_gives_a_well_formed_tree"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(true),
    },
];

/// Whether `elements` is a pre-order tree: every element's subtree lies inside the table and
/// inside its parent's, and the first element is its own parent.
fn well_formed(elements: &Elements) -> bool {
    let count = end_of(elements);
    positions(elements).all(|position| {
        let end = ryo::languages::last_at(elements, position);
        let parent = ryo::languages::parent_at(elements, position);
        end < count
            && (parent == position
                || (parent < position && end <= ryo::languages::last_at(elements, parent)))
    })
}

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check(adapter: &impl SyntaxPort) {
    run(CASES, Word::Succeeds, |text| {
        Ok(well_formed(&table_of(adapter, text)))
    });
}

/// The table the adapter reads `text` into.
fn table_of(adapter: &impl SyntaxPort, text: &str) -> Elements {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes()));
    adapter.read(SourceText(text.as_bytes()), &mut builder);
    elements
}
