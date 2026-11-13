//! The `implementations` service's cases: an impl's header without its generics and where clause, single-spaced, and where it is.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{service::kind, types::Kind};
use crate::outbound::syntax::{
    factory,
    interface::SyntaxPort as _,
    types::{Expanding, SourceText},
};
use ryo::languages;

/// The source a row parses.
type Input = &'static str;
/// What the service answered over it, written out.
type Value = &'static str;
/// Reading a tree is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_trait_impl"),
        word: Word::Succeeds,
        input: || "impl Display for S {}",
        expect: Expect::Value("[Display for S] 0..19"),
    },
    Case {
        name: CaseName("generics_and_where_clause_are_left_out"),
        word: Word::Succeeds,
        input: || "impl<T: Clone> Trait for Vec<T> where T: Copy {}",
        expect: Expect::Value("[Trait for Vec] 0..46"),
    },
    Case {
        name: CaseName("an_inherent_impl"),
        word: Word::Succeeds,
        input: || "impl   S { fn f() {} }",
        expect: Expect::Value("[S] 0..9"),
    },
];

/// The first element of the tree of kind `node`.
fn first(tree: &languages::Tree<'_>, node: Node) -> Option<languages::Position> {
    languages::positions(tree.elements).find(|at| kind(tree, *at) == Kind::Node(node))
}

/// The text of a name, or `-` for the unknown one.
fn text(
    tree: &languages::Tree<'_>,
    symbols: &languages::Symbols,
    name: languages::Symbol,
) -> String {
    languages::text_of(symbols, tree.source, name)
        .map_or_else(|| String::from("-"), |name| name.0.to_owned())
}

/// Parses the row's source and writes out what the service answers over its tree.
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
    let tree = &tree;
    let symbols = &mut symbols;
    let answer: String = {
        let implementation = first(tree, Node::Impl).expect("the row writes an impl");
        super::service::implemented(tree, symbols, implementation).map_or_else(
            || String::from("none"),
            |header| {
                format!(
                    "[{}] {}..{}",
                    text(tree, symbols, header.name),
                    header.from.0,
                    header.to.0
                )
            },
        )
    };
    Ok(Box::leak(answer.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
