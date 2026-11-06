//! The `fields` service's cases for jsx: the role of an edge of the tree under its parent.

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
        name: CaseName("attributes_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                r#"import a from "a" with { type: "json" };"#,
                Dialect::Plain,
                Kind::Node(Node::ImportDeclaration),
                Kind::Node(Node::ImportAttribute),
                0,
            )
        },
        expect: Expect::Value("attributes"),
    },
    Case {
        name: CaseName("children_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "<a><b /></a>;",
                Dialect::Jsx,
                Kind::Node(Node::JSXElement),
                Kind::Node(Node::JSXElement),
                0,
            )
        },
        expect: Expect::Value("children"),
    },
    Case {
        name: CaseName("closing_element_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "<a></a>;",
                Dialect::Jsx,
                Kind::Node(Node::JSXElement),
                Kind::Node(Node::JSXClosingElement),
                0,
            )
        },
        expect: Expect::Value("closing-element"),
    },
    Case {
        name: CaseName("closing_fragment_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "<></>;",
                Dialect::Jsx,
                Kind::Node(Node::JSXFragment),
                Kind::Node(Node::JSXClosingFragment),
                0,
            )
        },
        expect: Expect::Value("closing-fragment"),
    },
    Case {
        name: CaseName("opening_element_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "<a></a>;",
                Dialect::Jsx,
                Kind::Node(Node::JSXElement),
                Kind::Node(Node::JSXOpeningElement),
                0,
            )
        },
        expect: Expect::Value("opening-element"),
    },
    Case {
        name: CaseName("opening_fragment_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "<></>;",
                Dialect::Jsx,
                Kind::Node(Node::JSXFragment),
                Kind::Node(Node::JSXOpeningFragment),
                0,
            )
        },
        expect: Expect::Value("opening-fragment"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
