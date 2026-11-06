//! The `fields` service's cases for control: the role of an edge of the tree under its parent.

use super::fixtures::{Dialect, Edge, edge, subject};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// What a row is given.
type Input = Edge;
/// The name of the role the edge plays.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("none_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "x;",
                Dialect::Plain,
                Kind::Node(Node::ExpressionStatement),
                Kind::Token(Token::Punctuator),
                0,
            )
        },
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("alternate_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a ? b : c;",
                Dialect::Plain,
                Kind::Node(Node::ConditionalExpression),
                Kind::Node(Node::Identifier),
                2,
            )
        },
        expect: Expect::Value("alternate"),
    },
    Case {
        name: CaseName("block_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "try {} finally {}",
                Dialect::Plain,
                Kind::Node(Node::TryStatement),
                Kind::Node(Node::BlockStatement),
                0,
            )
        },
        expect: Expect::Value("block"),
    },
    Case {
        name: CaseName("body_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "while (a) {}",
                Dialect::Plain,
                Kind::Node(Node::WhileStatement),
                Kind::Node(Node::BlockStatement),
                0,
            )
        },
        expect: Expect::Value("body"),
    },
    Case {
        name: CaseName("cases_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "switch (a) { case 1: }",
                Dialect::Plain,
                Kind::Node(Node::SwitchStatement),
                Kind::Node(Node::SwitchCase),
                0,
            )
        },
        expect: Expect::Value("cases"),
    },
    Case {
        name: CaseName("consequent_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "if (a) b;",
                Dialect::Plain,
                Kind::Node(Node::IfStatement),
                Kind::Node(Node::ExpressionStatement),
                0,
            )
        },
        expect: Expect::Value("consequent"),
    },
    Case {
        name: CaseName("discriminant_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "switch (a) {}",
                Dialect::Plain,
                Kind::Node(Node::SwitchStatement),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("discriminant"),
    },
    Case {
        name: CaseName("finalizer_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "try {} finally {}",
                Dialect::Plain,
                Kind::Node(Node::TryStatement),
                Kind::Node(Node::BlockStatement),
                1,
            )
        },
        expect: Expect::Value("finalizer"),
    },
    Case {
        name: CaseName("handler_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "try {} catch {}",
                Dialect::Plain,
                Kind::Node(Node::TryStatement),
                Kind::Node(Node::CatchClause),
                0,
            )
        },
        expect: Expect::Value("handler"),
    },
    Case {
        name: CaseName("label_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a: for (;;) break a;",
                Dialect::Plain,
                Kind::Node(Node::BreakStatement),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("label"),
    },
    Case {
        name: CaseName("test_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "for (; a;) {}",
                Dialect::Plain,
                Kind::Node(Node::ForStatement),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("test"),
    },
    Case {
        name: CaseName("update_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "for (;; a++) {}",
                Dialect::Plain,
                Kind::Node(Node::ForStatement),
                Kind::Node(Node::UpdateExpression),
                0,
            )
        },
        expect: Expect::Value("update"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
