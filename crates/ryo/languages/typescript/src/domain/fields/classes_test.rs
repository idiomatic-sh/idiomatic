//! The `fields` service's cases for classes: the role of an edge of the tree under its parent.

use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use super::fixtures::{Dialect, Edge, edge, subject};

/// What a row is given.
type Input = Edge;
/// The name of the role the edge plays.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("decorators_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "@d class A {}",
                Dialect::Plain,
                Kind::Node(Node::ClassDeclaration),
                Kind::Node(Node::Decorator),
                0,
            )
        },
        expect: Expect::Value("decorators"),
    },
    Case {
        name: CaseName("default_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "type T<A = B> = A;",
                Dialect::Plain,
                Kind::Node(Node::TSTypeParameter),
                Kind::Node(Node::TSTypeReference),
                0,
            )
        },
        expect: Expect::Value("default"),
    },
    Case {
        name: CaseName("implements_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "class A implements B {}",
                Dialect::Plain,
                Kind::Node(Node::ClassDeclaration),
                Kind::Node(Node::TSClassImplements),
                0,
            )
        },
        expect: Expect::Value("implements"),
    },
    Case {
        name: CaseName("key_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "({ a: 1 });",
                Dialect::Plain,
                Kind::Node(Node::Property),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("key"),
    },
    Case {
        name: CaseName("meta_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "import.meta;",
                Dialect::Plain,
                Kind::Node(Node::MetaProperty),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("meta"),
    },
    Case {
        name: CaseName("name_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "type T<A> = A;",
                Dialect::Plain,
                Kind::Node(Node::TSTypeParameter),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("name"),
    },
    Case {
        name: CaseName("object_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a.b;",
                Dialect::Plain,
                Kind::Node(Node::MemberExpression),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("object"),
    },
    Case {
        name: CaseName("property_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a.b;",
                Dialect::Plain,
                Kind::Node(Node::MemberExpression),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("property"),
    },
    Case {
        name: CaseName("super_class_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "class A extends B {}",
                Dialect::Plain,
                Kind::Node(Node::ClassDeclaration),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("super-class"),
    },
    Case {
        name: CaseName("value_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "({ a: 1 });",
                Dialect::Plain,
                Kind::Node(Node::Property),
                Kind::Node(Node::Literal),
                0,
            )
        },
        expect: Expect::Value("value"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
