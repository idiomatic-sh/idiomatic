//! The `fields` service's cases for positions: the role of an edge of the tree under its parent.

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
        name: CaseName("an_arrow_body_follows_its_arrow"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a => b;",
                Kind::Node(Node::ArrowFunctionExpression),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("body"),
    },
    Case {
        name: CaseName("an_arrow_parameter_comes_before_its_arrow"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a => b;",
                Kind::Node(Node::ArrowFunctionExpression),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("params"),
    },
    Case {
        name: CaseName("a_for_initializer_comes_before_the_first_semicolon"),
        word: Word::Succeeds,
        input: || {
            edge(
                "for (a;;) {}",
                Kind::Node(Node::ForStatement),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("init"),
    },
    Case {
        name: CaseName("a_class_name_comes_before_extends"),
        word: Word::Succeeds,
        input: || {
            edge(
                "class A extends B {}",
                Kind::Node(Node::ClassDeclaration),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("id"),
    },
    Case {
        name: CaseName("a_method_value_is_its_function"),
        word: Word::Succeeds,
        input: || {
            edge(
                "class A { m() {} }",
                Kind::Node(Node::MethodDefinition),
                Kind::Node(Node::FunctionExpression),
                0,
            )
        },
        expect: Expect::Value("value"),
    },
    Case {
        name: CaseName("a_method_key_is_its_name"),
        word: Word::Succeeds,
        input: || {
            edge(
                "class A { m() {} }",
                Kind::Node(Node::MethodDefinition),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("key"),
    },
    Case {
        name: CaseName("a_unary_operator_is_its_first_token"),
        word: Word::Succeeds,
        input: || {
            edge(
                "-x;",
                Kind::Node(Node::UnaryExpression),
                Kind::Token(Token::Punctuator),
                0,
            )
        },
        expect: Expect::Value("operator"),
    },
    Case {
        name: CaseName("a_parenthesis_before_a_left_operand_is_no_operator"),
        word: Word::Succeeds,
        input: || {
            edge(
                "(a) + b;",
                Kind::Node(Node::BinaryExpression),
                Kind::Token(Token::Punctuator),
                0,
            )
        },
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("a_case_test_comes_before_its_colon"),
        word: Word::Succeeds,
        input: || {
            edge(
                "switch (a) { case 1: b; }",
                Kind::Node(Node::SwitchCase),
                Kind::Node(Node::Literal),
                0,
            )
        },
        expect: Expect::Value("test"),
    },
    Case {
        name: CaseName("a_case_statement_follows_its_colon"),
        word: Word::Succeeds,
        input: || {
            edge(
                "switch (a) { case 1: b; }",
                Kind::Node(Node::SwitchCase),
                Kind::Node(Node::ExpressionStatement),
                0,
            )
        },
        expect: Expect::Value("consequent"),
    },
    Case {
        name: CaseName("a_shorthand_property_value_is_its_second_identifier"),
        word: Word::Succeeds,
        input: || {
            edge(
                "({ a });",
                Kind::Node(Node::Property),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("value"),
    },
    Case {
        name: CaseName("a_shorthand_export_names_its_local_first"),
        word: Word::Succeeds,
        input: || {
            edge(
                "export { a };",
                Kind::Node(Node::ExportSpecifier),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("local"),
    },
    Case {
        name: CaseName("a_shorthand_default_is_the_value_of_its_property"),
        word: Word::Succeeds,
        input: || {
            edge(
                "const { a = 1 } = o;",
                Kind::Node(Node::Property),
                Kind::Node(Node::AssignmentPattern),
                0,
            )
        },
        expect: Expect::Value("value"),
    },
    Case {
        name: CaseName("a_shorthand_name_beside_a_default_is_the_key"),
        word: Word::Succeeds,
        input: || {
            edge(
                "const { a = 1 } = o;",
                Kind::Node(Node::Property),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("key"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
