//! The `fields` service's cases: the role of an edge of the tree under its parent, one row for each role of the vocabulary, on source
//! read through the parse. The roles are the properties of TypeScript-ESTree that hold a child (`@typescript-eslint/visitor-keys`).

use super::fixtures::{Dialect, Edge, edge};
use super::service::fields;
use super::types::Field;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

#[cfg(test)]
mod classes_test;
#[cfg(test)]
mod control_test;
#[cfg(test)]
mod jsx_test;
#[cfg(test)]
mod modules_test;
#[cfg(test)]
mod positions_test;
#[cfg(test)]
mod types_test;

/// What a row is given.
type Input = Edge;
/// The name of the role the edge plays.
type Value = &'static str;
type Failure = ();

/// Reads one row's input through the concept fixtures' shared reading.
fn subject(input: Input) -> Result<Value, Failure> {
    super::fixtures::subject(input)
}

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("argument_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "-x;",
                Dialect::Plain,
                Kind::Node(Node::UnaryExpression),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("argument"),
    },
    Case {
        name: CaseName("arguments_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "f(a);",
                Dialect::Plain,
                Kind::Node(Node::CallExpression),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("arguments"),
    },
    Case {
        name: CaseName("callee_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "f(a);",
                Dialect::Plain,
                Kind::Node(Node::CallExpression),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("callee"),
    },
    Case {
        name: CaseName("expression_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a;",
                Dialect::Plain,
                Kind::Node(Node::ExpressionStatement),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("expression"),
    },
    Case {
        name: CaseName("expressions_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a, b;",
                Dialect::Plain,
                Kind::Node(Node::SequenceExpression),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("expressions"),
    },
    Case {
        name: CaseName("left_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a + b;",
                Dialect::Plain,
                Kind::Node(Node::BinaryExpression),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("left"),
    },
    Case {
        name: CaseName("operator_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a + b;",
                Dialect::Plain,
                Kind::Node(Node::BinaryExpression),
                Kind::Token(Token::Punctuator),
                0,
            )
        },
        expect: Expect::Value("operator"),
    },
    Case {
        name: CaseName("options_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                r#"import("m", { with: {} });"#,
                Dialect::Plain,
                Kind::Node(Node::ImportExpression),
                Kind::Node(Node::ObjectExpression),
                0,
            )
        },
        expect: Expect::Value("options"),
    },
    Case {
        name: CaseName("param_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "try {} catch (e) {}",
                Dialect::Plain,
                Kind::Node(Node::CatchClause),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("param"),
    },
    Case {
        name: CaseName("params_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "function f(a) {}",
                Dialect::Plain,
                Kind::Node(Node::FunctionDeclaration),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("params"),
    },
    Case {
        name: CaseName("properties_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "({ a });",
                Dialect::Plain,
                Kind::Node(Node::ObjectExpression),
                Kind::Node(Node::Property),
                0,
            )
        },
        expect: Expect::Value("properties"),
    },
    Case {
        name: CaseName("quasi_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "f`a`;",
                Dialect::Plain,
                Kind::Node(Node::TaggedTemplateExpression),
                Kind::Node(Node::TemplateLiteral),
                0,
            )
        },
        expect: Expect::Value("quasi"),
    },
    Case {
        name: CaseName("quasis_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "`a`;",
                Dialect::Plain,
                Kind::Node(Node::TemplateLiteral),
                Kind::Node(Node::TemplateElement),
                0,
            )
        },
        expect: Expect::Value("quasis"),
    },
    Case {
        name: CaseName("right_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a + b;",
                Dialect::Plain,
                Kind::Node(Node::BinaryExpression),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("right"),
    },
    Case {
        name: CaseName("tag_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "f`a`;",
                Dialect::Plain,
                Kind::Node(Node::TaggedTemplateExpression),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("tag"),
    },
];

/// The rows of every table of the part's cases.
fn all_cases() -> impl Iterator<Item = &'static Case<Input, Value, Failure>> {
    [
        CASES,
        positions_test::CASES,
        jsx_test::CASES,
        modules_test::CASES,
        control_test::CASES,
        types_test::CASES,
        classes_test::CASES,
    ]
    .into_iter()
    .flat_map(|table| table.iter())
}

/// Every role of the vocabulary has a row of its own.
fn every_role_has_a_row() -> usize {
    fields()
        .filter(|field| {
            let name = super::service::field_name(*field).0;
            !all_cases().any(|case| matches!(case.expect, Expect::Value(wanted) if wanted == name))
        })
        .count()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    assert_eq!(every_role_has_a_row(), 0, "{:?}", Field::None);
}
