//! The `arguments` service's cases: the arguments a call writes, and the name each hands on.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{
    service::kind,
    types::{Dialect, Kind},
};
use crate::outbound::syntax::{factory, interface::SyntaxPort as _, types::SourceText};
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
        name: CaseName("plain_and_spread_arguments_hand_on_their_names"),
        word: Word::Succeeds,
        input: || "f(a, ...b, 1);",
        expect: Expect::Value("Node(Identifier)=a Node(SpreadElement)=b Node(Literal)=-"),
    },
    Case {
        name: CaseName("type_arguments_are_no_argument"),
        word: Word::Succeeds,
        input: || "f<T>(c);",
        expect: Expect::Value("Node(Identifier)=c"),
    },
    Case {
        name: CaseName("a_new_writes_its_arguments"),
        word: Word::Succeeds,
        input: || "new C(d, e);",
        expect: Expect::Value("Node(Identifier)=d Node(Identifier)=e"),
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
    let tree = &tree;
    let symbols = &mut symbols;
    let answer: String = {
        let call = first(tree, Node::CallExpression)
            .or_else(|| first(tree, Node::NewExpression))
            .expect("the row writes a call");
        super::service::arguments_of(tree, call)
            .map(|argument| {
                let name = super::service::passed(tree, symbols, argument);
                format!("{:?}={}", kind(tree, argument), text(tree, symbols, name))
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
