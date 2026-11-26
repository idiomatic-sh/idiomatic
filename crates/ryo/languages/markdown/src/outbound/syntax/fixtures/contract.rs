//! What every `syntax` adapter must pass.

use crate::outbound::syntax::types::SourceText;
use ryo::languages::{NodeTable, Position, node_table};

use super::super::interface::SyntaxPort;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract is given: Markdown text.
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

/// Whether `table` is a tree: the first element is its own parent, and every other element has a parent among the elements.
fn well_formed(table: &NodeTable) -> bool {
    let count = u32::try_from(node_table::element_count(table).0).unwrap_or(u32::MAX);
    (0..count).map(Position).all(|at| {
        let parent = node_table::parent_of(table, at);
        if at == Position(0) { parent == at } else { parent != at && parent.0 < count }
    })
}

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check(adapter: &impl SyntaxPort) {
    run(CASES, Word::Succeeds, |text| Ok(with_table(adapter, text, well_formed)));
}

/// What `look` says of the node table the adapter walks `text` into.
fn with_table<T>(adapter: &impl SyntaxPort, text: &str, look: impl FnOnce(&NodeTable) -> T) -> T {
    let mut table = NodeTable::default();
    let mut builder = node_table::builder(&mut table, ryo::languages::Count(64));
    adapter.walk(
        SourceText(text.as_bytes()),
        ryo::languages::SourcePath("a.md"),
        &mut crate::domain::file_view::types::Decodings::default(),
        &mut builder,
    );
    look(node_table::finished(&mut builder))
}

/// Every element of the table the adapter walks `source` into, in the order of the tree (each before its children): its kind and the
/// text it spans.
#[must_use]
pub(crate) fn elements_of(
    adapter: &impl SyntaxPort,
    source: &str,
) -> Vec<(crate::domain::syntax::types::Kind, String)> {
    with_table(adapter, source, |table| {
        let mut found = Vec::new();
        let mut pending = vec![Position(0)];
        while let Some(at) = pending.pop() {
            let span = node_table::span_of(table, at);
            let (start, end) = (usize::try_from(span.start.0).unwrap_or(0), usize::try_from(span.end.0).unwrap_or(0));
            found.push((
                crate::domain::syntax::service::kind_of(node_table::kind_of(table, at)),
                source.get(start..end).unwrap_or_default().to_owned(),
            ));
            pending.extend(node_table::children_of(table, at).iter().rev().copied());
        }
        found
    })
}
