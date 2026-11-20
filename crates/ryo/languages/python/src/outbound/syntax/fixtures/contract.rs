//! What every `syntax` adapter must pass.

use crate::outbound::syntax::types::SourceText;
use ryo::languages::{
    ByteSlice, Elements, Position, Symbols, end_at, end_of, kind_at, positions, start_at,
    table_builder,
};

use super::super::interface::SyntaxPort;
use super::super::types::PythonSourceKind;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract is given: Python text.
type Input = &'static str;
/// Whether the table is a tree: one root, and every element's subtree inside its parent's.
type Value = bool;
/// A syntax port refuses nothing: the parser recovers from what it cannot read.
type Failure = ();

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_assignment_gives_a_well_formed_tree"),
        word: Word::Succeeds,
        input: || "name = \"demo\"\n",
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
        let (Some(size), Some(parent)) = (
            entry(&elements.size, position),
            entry(&elements.parent, position),
        ) else {
            return false;
        };
        let end = Position(position.0 + size.0);
        end < count
            && (parent == position
                || (parent < position
                    && entry(&elements.size, parent)
                        .is_some_and(|above| end <= Position(parent.0 + above.0))))
    })
}

/// The entry of `column` at `position`; none past the end of the table, which a malformed tree may point to.
fn entry<T>(column: &[T], position: Position) -> Option<T>
where
    T: Copy,
{
    column.get(usize::try_from(position.0).ok()?).copied()
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
    run(CASES, Word::Succeeds, |text| Ok(walked_well(adapter, text)));
    drop(adapter.begin_semantics(
        &Elements::default(),
        PythonSourceKind::Python,
        ryo::languages::SemanticSlot::default(),
    ));
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    {
        let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(&[]));
        adapter.expand_annotations(SourceText(&[]), &[], &mut builder);
    }
    assert_eq!(end_of(&elements), end_of(&Elements::default()));
}

/// Whether the adapter walks `text` into a table that keeps the laws of the engine's node table, and lends its tree once.
fn walked_well(adapter: &impl SyntaxPort, text: &str) -> bool {
    use crate::fixtures::table_laws::broken_laws;
    use ryo::languages::{Count, NodeTable, Offset, node_table};
    let kinds = Count(crate::domain::syntax::service::kinds().count());
    let length = Offset(u32::try_from(text.len()).unwrap_or(u32::MAX));
    let mut table = NodeTable::default();
    let mut builder = node_table::builder(&mut table, kinds);
    let (mut lent, mut broken) = (0_usize, 0_usize);
    adapter.walk(
        SourceText(text.as_bytes()),
        PythonSourceKind::Python,
        crate::outbound::syntax::types::ReportedSyntax::Every,
        &mut builder,
        &mut |walked| {
            // This reader reads no string as a type expression: the tree it is lent is the file's own.
            if let crate::domain::node::types::Walked::Whole(written, _tree) = walked {
                lent += 1;
                broken += broken_laws(node_table::finished(written), kinds, length).len();
            }
            Vec::new()
        },
    );
    lent == 1 && broken == 0
}

/// The table the adapter reads `text` into.
fn table_of(adapter: &impl SyntaxPort, text: &str) -> Elements {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes()));
    adapter.read(
        SourceText(text.as_bytes()),
        PythonSourceKind::Python,
        &mut builder,
    );
    elements
}

/// Every element of the table the adapter reads `source` into, in order: its kind and the text it spans.
#[must_use]
pub(crate) fn elements_of(
    adapter: &impl SyntaxPort,
    source: &str,
) -> Vec<(crate::domain::syntax::types::Kind, String)> {
    let elements = table_of(adapter, source);
    positions(&elements)
        .map(|position| {
            let (start, end) = (
                usize::try_from(start_at(&elements, position).0).unwrap_or(0),
                usize::try_from(end_at(&elements, position).0).unwrap_or(0),
            );
            (
                crate::domain::syntax::service::kind_of(kind_at(&elements, position)),
                source.get(start..end).unwrap_or_default().to_owned(),
            )
        })
        .collect()
}
