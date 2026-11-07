//! The `fields` service's cases for control: the role of an edge of the tree under its parent.

use super::fixtures::{Edge, edge, subject};
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
