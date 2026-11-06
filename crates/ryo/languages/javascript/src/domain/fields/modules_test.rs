//! The `fields` service's cases for modules: the role of an edge of the tree under its parent.

use super::fixtures::{Edge, edge, subject};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;

/// What a row is given.
type Input = Edge;
/// The name of the role the edge plays.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("declaration_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "export const a = 1;",
                Kind::Node(Node::ExportNamedDeclaration),
                Kind::Node(Node::VariableDeclaration),
                0,
            )
        },
        expect: Expect::Value("declaration"),
    },
    Case {
        name: CaseName("declarations_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "let a = 1;",
                Kind::Node(Node::VariableDeclaration),
                Kind::Node(Node::VariableDeclarator),
                0,
            )
        },
        expect: Expect::Value("declarations"),
    },
    Case {
        name: CaseName("exported_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "export { a as b };",
                Kind::Node(Node::ExportSpecifier),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("exported"),
    },
    Case {
        name: CaseName("id_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "function f() {}",
                Kind::Node(Node::FunctionDeclaration),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("id"),
    },
    Case {
        name: CaseName("imported_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                r#"import { a as b } from "m";"#,
                Kind::Node(Node::ImportSpecifier),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("imported"),
    },
    Case {
        name: CaseName("init_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "let a = 1;",
                Kind::Node(Node::VariableDeclarator),
                Kind::Node(Node::Literal),
                0,
            )
        },
        expect: Expect::Value("init"),
    },
    Case {
        name: CaseName("local_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                r#"import a from "m";"#,
                Kind::Node(Node::ImportDefaultSpecifier),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("local"),
    },
    Case {
        name: CaseName("namespace_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "<a:b />;",
                Kind::Node(Node::JSXNamespacedName),
                Kind::Node(Node::JSXIdentifier),
                0,
            )
        },
        expect: Expect::Value("namespace"),
    },
    Case {
        name: CaseName("source_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                r#"import a from "m";"#,
                Kind::Node(Node::ImportDeclaration),
                Kind::Node(Node::Literal),
                0,
            )
        },
        expect: Expect::Value("source"),
    },
    Case {
        name: CaseName("specifiers_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                r#"import a from "m";"#,
                Kind::Node(Node::ImportDeclaration),
                Kind::Node(Node::ImportDefaultSpecifier),
                0,
            )
        },
        expect: Expect::Value("specifiers"),
    },
    Case {
        name: CaseName("super_class_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "class A extends B {}",
                Kind::Node(Node::ClassDeclaration),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("super-class"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
