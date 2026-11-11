//! The `scopes` service's cases: the scope each name is written in, and the node that owns it.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{
    service::kind,
    types::{Goal, Kind},
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
        name: CaseName("blocks_and_functions_open_scopes"),
        word: Word::Succeeds,
        input: || "function f() { let a; { let b; } }",
        expect: Expect::Value(
            "Node(Identifier)@9 in Node(FunctionDeclaration)@0 owned by 1 Node(Identifier)@19 in Node(BlockStatement)@13 owned by 9 Node(Identifier)@28 in Node(BlockStatement)@22 owned by 20",
        ),
    },
    Case {
        name: CaseName("a_method_body_is_its_own_scope"),
        word: Word::Succeeds,
        input: || "class C { m() { x; } }",
        expect: Expect::Value(
            "Node(Identifier)@6 in Node(ClassDeclaration)@0 owned by 1 Node(Identifier)@10 in Node(ClassDeclaration)@0 owned by 1 Node(Identifier)@16 in Node(BlockStatement)@14 owned by 17",
        ),
    },
    Case {
        name: CaseName("a_top_level_name_is_in_the_file"),
        word: Word::Succeeds,
        input: || "y;",
        expect: Expect::Value("Node(Identifier)@0 in Node(Program)@0 owned by 0"),
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
    factory::oxc_parser().read(
        SourceText(source.as_bytes()),
        Goal::Module,
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
            .filter(|each| kind(tree, *each) == Kind::Node(Node::Identifier))
            .map(|name| {
                format!(
                    "{} in {} owned by {}",
                    at(tree, name),
                    at(tree, super::service::enclosing(tree, name)),
                    super::service::owner(tree, name).0
                )
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
