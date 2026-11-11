//! The `resolutions` service's cases: the one name a type path writes, what binds it there, and what holds it.

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
        name: CaseName("a_struct_binds_a_reference_in_a_sibling_function"),
        word: Word::Succeeds,
        input: || "struct A; fn f(x: A) {}",
        expect: Expect::Value("A->Node(Struct)@0 held by 8"),
    },
    Case {
        name: CaseName("a_type_parameter_binds_inside_its_function"),
        word: Word::Succeeds,
        input: || "fn g<T>(y: T) {}",
        expect: Expect::Value("T->Node(TypeParam)@5 held by 1"),
    },
    Case {
        name: CaseName("a_qualified_path_has_no_plain_name"),
        word: Word::Succeeds,
        input: || "fn h(z: m::A) {}",
        expect: Expect::Value("no plain name held by 1"),
    },
    Case {
        name: CaseName("an_unbound_name_binds_nothing"),
        word: Word::Succeeds,
        input: || "fn k(w: Missing) {}",
        expect: Expect::Value("Missing->unbound held by 1"),
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
    let symbols = &mut symbols;
    let answer: String = {
        let reference = first(tree, Node::PathType).expect("the row writes a type path");
        let holder = super::service::holder(tree, reference);
        let bound = super::service::plain_type_name(tree, reference).map_or_else(
            || String::from("no plain name"),
            |name| {
                let binding = super::service::binding_of(tree, reference, name)
                    .map_or_else(|| String::from("unbound"), |each| at(tree, each));
                format!("{}->{binding}", text(tree, symbols, name))
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
