//! The `visibilities` service's cases: the leaves each visibility is written as, for an item and for a use.

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
        name: CaseName("a_pub_function"),
        word: Word::Succeeds,
        input: || "pub fn f() {}",
        expect: Expect::Value("Node(Fn)@0"),
    },
    Case {
        name: CaseName("a_crate_visible_struct"),
        word: Word::Succeeds,
        input: || "pub(crate) struct S;",
        expect: Expect::Value("Node(Struct)@0"),
    },
    Case {
        name: CaseName("a_pub_use"),
        word: Word::Succeeds,
        input: || "pub use a::b;",
        expect: Expect::Value("Node(UseTree)@8"),
    },
];

/// An element written out: its kind and where it starts.
fn at(tree: &languages::Tree<'_>, element: languages::Position) -> String {
    format!(
        "{:?}@{}",
        kind(tree, element),
        languages::start_at(tree.elements, element).0
    )
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
    let answer: String = {
        languages::positions(tree.elements)
            .filter(|each| kind(tree, *each) == Kind::Node(Node::Visibility))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|visibility| {
                super::service::visible(tree, visibility)
                    .map(|leaf| at(tree, leaf))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    Ok(Box::leak(answer.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
