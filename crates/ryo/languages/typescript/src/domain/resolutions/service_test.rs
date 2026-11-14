//! The `resolutions` service's cases: the plain name a type reference writes, and what holds it.

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
        name: CaseName("a_reference_in_a_function_is_held_by_the_function"),
        word: Word::Succeeds,
        input: || "class A {} function f(x: A) {}",
        expect: Expect::Value("A held by 11"),
    },
    Case {
        name: CaseName("a_type_parameter_reference_is_held_by_its_function"),
        word: Word::Succeeds,
        input: || "function g<T>(y: T) {}",
        expect: Expect::Value("T held by 1"),
    },
    Case {
        name: CaseName("a_qualified_reference_has_no_plain_name"),
        word: Word::Succeeds,
        input: || "function h(z: N.A) {}",
        expect: Expect::Value("no plain name held by 1"),
    },
    Case {
        name: CaseName("a_reference_outside_any_declaration_is_held_by_the_file"),
        word: Word::Succeeds,
        input: || "let w: Missing;",
        expect: Expect::Value("Missing held by 0"),
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
        let reference =
            first(tree, Node::TSTypeReference).expect("the row writes a type reference");
        let holder = super::service::holder(tree, reference);
        let bound = super::service::plain_type_name(tree, reference).map_or_else(
            || String::from("no plain name"),
            |identifier| {
                let name = crate::domain::names::service::identifier(tree, symbols, identifier);
                text(tree, symbols, name)
            },
        );
        format!("{bound} held by {}", holder.0)
    };
    Ok(Box::leak(answer.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
