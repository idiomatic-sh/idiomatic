//! The `imports` service's cases, on trees written by hand: the leaves of a `use`, each by its text.

use crate::domain::nodes::types::RustNode;
use ryo::languages::{self, Position, Tree};

use super::service::leaves;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::{end, kind, start};
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::fixtures::script::table_of;
use crate::outbound::syntax::fixtures::trees::tree_of;

/// The source of a `use`.
type Input = &'static str;
/// The text of each leaf, in source order.
type Value = &'static [&'static str];
type Failure = ();

/// The text of the element at `at`.
fn text(tree: &Tree<'_>, at: Position) -> &'static str {
    let (from, to) = (
        usize::try_from(start(tree, at).0).unwrap_or(0),
        usize::try_from(end(tree, at).0).unwrap_or(0),
    );
    Box::leak(
        String::from_utf8_lossy(tree.source.0.get(from..to).unwrap_or_default())
            .into_owned()
            .into_boxed_str(),
    )
}

fn subject(source: Input) -> Result<Value, Failure> {
    let (elements, _symbols, _) =
        table_of(&tree_of(source).expect("a tree is written for the source"));
    let tree = Tree {
        elements: &elements,
        source: languages::ByteSlice(source.as_bytes()),
        file: languages::FileId(0),
    };
    let root = languages::positions(tree.elements)
        .find(|at| kind(&tree, *at) == Kind::Node(RustNode::UseTree))
        .ok_or(())?;
    let found: Vec<&'static str> = leaves(&tree, root).map(|leaf| text(&tree, leaf)).collect();
    Ok(Box::leak(found.into_boxed_slice()))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_group_has_a_leaf_for_each_name"),
        word: Word::Succeeds,
        input: || "use a::{b, c, d};",
        expect: Expect::Value(&["b", "c", "d"]),
    },
    Case {
        name: CaseName("a_plain_use_is_its_own_leaf"),
        word: Word::Succeeds,
        input: || "use a::b;",
        expect: Expect::Value(&["a::b"]),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
