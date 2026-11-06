//! The `fields` service's cases for positions: the role of an edge of the tree under its parent.

use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
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
        name: CaseName("the_property_of_new_target_is_the_property_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "function f() { new.target; }",
                Dialect::Plain,
                Kind::Node(Node::MetaProperty),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("property"),
    },
    Case {
        name: CaseName("an_arrow_body_follows_its_arrow"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a => b;",
                Dialect::Plain,
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
                Dialect::Plain,
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
                Dialect::Plain,
                Kind::Node(Node::ForStatement),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("init"),
    },
    Case {
        name: CaseName("a_mapped_type_constraint_follows_in"),
        word: Word::Succeeds,
        input: || {
            edge(
                "type T = { [K in A]: C };",
                Dialect::Plain,
                Kind::Node(Node::TSMappedType),
                Kind::Node(Node::TSTypeReference),
                0,
            )
        },
        expect: Expect::Value("constraint"),
    },
    Case {
        name: CaseName("a_mapped_type_value_follows_its_colon"),
        word: Word::Succeeds,
        input: || {
            edge(
                "type T = { [K in A]: C };",
                Dialect::Plain,
                Kind::Node(Node::TSMappedType),
                Kind::Node(Node::TSTypeReference),
                1,
            )
        },
        expect: Expect::Value("type-annotation"),
    },
    Case {
        name: CaseName("a_class_name_comes_before_extends"),
        word: Word::Succeeds,
        input: || {
            edge(
                "class A extends B {}",
                Dialect::Plain,
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
                Dialect::Plain,
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
                Dialect::Plain,
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
                Dialect::Plain,
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
                Dialect::Plain,
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
                Dialect::Plain,
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
                Dialect::Plain,
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
                Dialect::Plain,
                Kind::Node(Node::Property),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("value"),
    },
    Case {
        name: CaseName("a_signature_annotation_is_its_return_type"),
        word: Word::Succeeds,
        input: || {
            edge(
                "interface A { m(): B }",
                Dialect::Plain,
                Kind::Node(Node::TSMethodSignature),
                Kind::Node(Node::TSTypeAnnotation),
                0,
            )
        },
        expect: Expect::Value("return-type"),
    },
    Case {
        name: CaseName("a_member_annotation_is_its_type"),
        word: Word::Succeeds,
        input: || {
            edge(
                "interface A { a: B }",
                Dialect::Plain,
                Kind::Node(Node::TSPropertySignature),
                Kind::Node(Node::TSTypeAnnotation),
                0,
            )
        },
        expect: Expect::Value("type-annotation"),
    },
    Case {
        name: CaseName("a_shorthand_export_names_its_exported_first"),
        word: Word::Succeeds,
        input: || {
            edge(
                "export { a };",
                Dialect::Plain,
                Kind::Node(Node::ExportSpecifier),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("exported"),
    },
    Case {
        name: CaseName("a_shorthand_default_is_the_value_of_its_property"),
        word: Word::Succeeds,
        input: || {
            edge(
                "const { a = 1 } = o;",
                Dialect::Plain,
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
                Dialect::Plain,
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
