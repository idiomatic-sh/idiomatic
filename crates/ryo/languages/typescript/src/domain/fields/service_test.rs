// one row for each role of the vocabulary, and the check that every role has one: the table is the proof, and splitting it would split that check
//! The `fields` service's cases: the role of an edge of the tree under its parent, one row for each role of the vocabulary, on source
//! read through the parse. The roles are the properties of TypeScript-ESTree that hold a child (`@typescript-eslint/visitor-keys`).

use std::sync::Arc;

use super::service::{fields, role_of};
use super::types::Field;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind, parent};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::syntax::factory::oxc_parser;
use ryo::languages::{self, Position, Tree};

/// Whether a source is read as TypeScript alone or with JSX.
#[derive(Clone, Copy)]
enum Dialect {
    /// A `.ts` file.
    Plain,
    /// A `.tsx` file.
    Jsx,
}

/// An edge of the tree of some source: the nth edge, in source order, from a node of a kind to an element of a kind.
#[derive(Clone, Copy)]
struct Edge {
    source: &'static str,
    dialect: Dialect,
    parent: Kind,
    child: Kind,
    nth: usize,
}

/// What a row is given.
type Input = Edge;
/// The name of the role the edge plays.
type Value = &'static str;
type Failure = ();

const fn edge(
    source: &'static str,
    dialect: Dialect,
    parent: Kind,
    child: Kind,
    nth: usize,
) -> Edge {
    Edge {
        source,
        dialect,
        parent,
        child,
        nth,
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
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
        name: CaseName("check_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "type T = A extends B ? C : D;",
                Dialect::Plain,
                Kind::Node(Node::TSConditionalType),
                Kind::Node(Node::TSTypeReference),
                0,
            )
        },
        expect: Expect::Value("check-type"),
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
        name: CaseName("constraint_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "type T<A extends B> = A;",
                Dialect::Plain,
                Kind::Node(Node::TSTypeParameter),
                Kind::Node(Node::TSTypeReference),
                0,
            )
        },
        expect: Expect::Value("constraint"),
    },
    Case {
        name: CaseName("declaration_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "export const a = 1;",
                Dialect::Plain,
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
                Dialect::Plain,
                Kind::Node(Node::VariableDeclaration),
                Kind::Node(Node::VariableDeclarator),
                0,
            )
        },
        expect: Expect::Value("declarations"),
    },
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
        name: CaseName("element_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "let a: B[];",
                Dialect::Plain,
                Kind::Node(Node::TSArrayType),
                Kind::Node(Node::TSTypeReference),
                0,
            )
        },
        expect: Expect::Value("element-type"),
    },
    Case {
        name: CaseName("element_types_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "let a: [B];",
                Dialect::Plain,
                Kind::Node(Node::TSTupleType),
                Kind::Node(Node::TSTypeReference),
                0,
            )
        },
        expect: Expect::Value("element-types"),
    },
    Case {
        name: CaseName("elements_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "[a];",
                Dialect::Plain,
                Kind::Node(Node::ArrayExpression),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("elements"),
    },
    Case {
        name: CaseName("exported_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "export { a as b };",
                Dialect::Plain,
                Kind::Node(Node::ExportSpecifier),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("exported"),
    },
    Case {
        name: CaseName("expr_name_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "let a: typeof b;",
                Dialect::Plain,
                Kind::Node(Node::TSTypeQuery),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("expr-name"),
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
        name: CaseName("extends_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "interface A extends B {}",
                Dialect::Plain,
                Kind::Node(Node::TSInterfaceDeclaration),
                Kind::Node(Node::TSInterfaceHeritage),
                0,
            )
        },
        expect: Expect::Value("extends"),
    },
    Case {
        name: CaseName("extends_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "type T = A extends B ? C : D;",
                Dialect::Plain,
                Kind::Node(Node::TSConditionalType),
                Kind::Node(Node::TSTypeReference),
                1,
            )
        },
        expect: Expect::Value("extends-type"),
    },
    Case {
        name: CaseName("false_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "type T = A extends B ? C : D;",
                Dialect::Plain,
                Kind::Node(Node::TSConditionalType),
                Kind::Node(Node::TSTypeReference),
                3,
            )
        },
        expect: Expect::Value("false-type"),
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
        name: CaseName("id_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "function f() {}",
                Dialect::Plain,
                Kind::Node(Node::FunctionDeclaration),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("id"),
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
        name: CaseName("imported_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                r#"import { a as b } from "m";"#,
                Dialect::Plain,
                Kind::Node(Node::ImportSpecifier),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("imported"),
    },
    Case {
        name: CaseName("index_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "let a: B[C];",
                Dialect::Plain,
                Kind::Node(Node::TSIndexedAccessType),
                Kind::Node(Node::TSTypeReference),
                1,
            )
        },
        expect: Expect::Value("index-type"),
    },
    Case {
        name: CaseName("init_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "let a = 1;",
                Dialect::Plain,
                Kind::Node(Node::VariableDeclarator),
                Kind::Node(Node::Literal),
                0,
            )
        },
        expect: Expect::Value("init"),
    },
    Case {
        name: CaseName("initializer_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "enum E { A = 1 }",
                Dialect::Plain,
                Kind::Node(Node::TSEnumMember),
                Kind::Node(Node::Literal),
                0,
            )
        },
        expect: Expect::Value("initializer"),
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
        name: CaseName("literal_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "let a: 1;",
                Dialect::Plain,
                Kind::Node(Node::TSLiteralType),
                Kind::Node(Node::Literal),
                0,
            )
        },
        expect: Expect::Value("literal"),
    },
    Case {
        name: CaseName("local_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                r#"import a from "m";"#,
                Dialect::Plain,
                Kind::Node(Node::ImportDefaultSpecifier),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("local"),
    },
    Case {
        name: CaseName("members_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "enum E { A }",
                Dialect::Plain,
                Kind::Node(Node::TSEnumBody),
                Kind::Node(Node::TSEnumMember),
                0,
            )
        },
        expect: Expect::Value("members"),
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
        name: CaseName("module_reference_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                r#"import a = require("m");"#,
                Dialect::Plain,
                Kind::Node(Node::TSImportEqualsDeclaration),
                Kind::Node(Node::TSExternalModuleReference),
                0,
            )
        },
        expect: Expect::Value("module-reference"),
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
        name: CaseName("name_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "type T = { [K in A as B]: C };",
                Dialect::Plain,
                Kind::Node(Node::TSMappedType),
                Kind::Node(Node::TSTypeReference),
                1,
            )
        },
        expect: Expect::Value("name-type"),
    },
    Case {
        name: CaseName("namespace_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "<a:b />;",
                Dialect::Jsx,
                Kind::Node(Node::JSXNamespacedName),
                Kind::Node(Node::JSXIdentifier),
                0,
            )
        },
        expect: Expect::Value("namespace"),
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
        name: CaseName("object_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "let a: B[C];",
                Dialect::Plain,
                Kind::Node(Node::TSIndexedAccessType),
                Kind::Node(Node::TSTypeReference),
                0,
            )
        },
        expect: Expect::Value("object-type"),
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
        name: CaseName("parameter_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "class A { constructor(private a) {} }",
                Dialect::Plain,
                Kind::Node(Node::TSParameterProperty),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("parameter"),
    },
    Case {
        name: CaseName("parameter_name_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "function f(a): a is B {}",
                Dialect::Plain,
                Kind::Node(Node::TSTypePredicate),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("parameter-name"),
    },
    Case {
        name: CaseName("parameters_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "interface A { [k: string]: B }",
                Dialect::Plain,
                Kind::Node(Node::TSIndexSignature),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("parameters"),
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
        name: CaseName("qualifier_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                r#"let a: import("m").B;"#,
                Dialect::Plain,
                Kind::Node(Node::TSImportType),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("qualifier"),
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
        name: CaseName("return_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "function f(): A {}",
                Dialect::Plain,
                Kind::Node(Node::FunctionDeclaration),
                Kind::Node(Node::TSTypeAnnotation),
                0,
            )
        },
        expect: Expect::Value("return-type"),
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
        name: CaseName("source_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                r#"import a from "m";"#,
                Dialect::Plain,
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
                Dialect::Plain,
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
                Dialect::Plain,
                Kind::Node(Node::ClassDeclaration),
                Kind::Node(Node::Identifier),
                1,
            )
        },
        expect: Expect::Value("super-class"),
    },
    Case {
        name: CaseName("super_type_arguments_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "class A extends B<C> {}",
                Dialect::Plain,
                Kind::Node(Node::ClassDeclaration),
                Kind::Node(Node::TSTypeParameterInstantiation),
                0,
            )
        },
        expect: Expect::Value("super-type-arguments"),
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
        name: CaseName("true_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "type T = A extends B ? C : D;",
                Dialect::Plain,
                Kind::Node(Node::TSConditionalType),
                Kind::Node(Node::TSTypeReference),
                2,
            )
        },
        expect: Expect::Value("true-type"),
    },
    Case {
        name: CaseName("type_annotation_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "let a: B;",
                Dialect::Plain,
                Kind::Node(Node::Identifier),
                Kind::Node(Node::TSTypeAnnotation),
                0,
            )
        },
        expect: Expect::Value("type-annotation"),
    },
    Case {
        name: CaseName("type_arguments_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "f<A>();",
                Dialect::Plain,
                Kind::Node(Node::CallExpression),
                Kind::Node(Node::TSTypeParameterInstantiation),
                0,
            )
        },
        expect: Expect::Value("type-arguments"),
    },
    Case {
        name: CaseName("type_name_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "let a: B;",
                Dialect::Plain,
                Kind::Node(Node::TSTypeReference),
                Kind::Node(Node::Identifier),
                0,
            )
        },
        expect: Expect::Value("type-name"),
    },
    Case {
        name: CaseName("type_parameter_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "type T<A> = A extends infer B ? B : A;",
                Dialect::Plain,
                Kind::Node(Node::TSInferType),
                Kind::Node(Node::TSTypeParameter),
                0,
            )
        },
        expect: Expect::Value("type-parameter"),
    },
    Case {
        name: CaseName("type_parameters_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "function f<A>() {}",
                Dialect::Plain,
                Kind::Node(Node::FunctionDeclaration),
                Kind::Node(Node::TSTypeParameterDeclaration),
                0,
            )
        },
        expect: Expect::Value("type-parameters"),
    },
    Case {
        name: CaseName("types_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "let a: B | C;",
                Dialect::Plain,
                Kind::Node(Node::TSUnionType),
                Kind::Node(Node::TSTypeReference),
                0,
            )
        },
        expect: Expect::Value("types"),
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

fn subject(given: Input) -> Result<Value, Failure> {
    let language = ParseContract::new(
        Arc::new(oxc_parser()),
        Vec::new(),
        Arc::new(crate::outbound::notes::factory::postcard()),
    );
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    languages::contract::build(
        &language,
        languages::ParseRequest {
            path: languages::SourcePath(match given.dialect {
                Dialect::Plain => "item.ts",
                Dialect::Jsx => "item.tsx",
            }),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(given.source.as_bytes()),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
    )
    .map_err(drop)?;
    let tree = Tree {
        elements: &elements,
        source: languages::ByteSlice(given.source.as_bytes()),
        file: languages::FileId(0),
    };
    let found = languages::positions(&elements)
        .filter(|at| *at != Position(0))
        .filter(|at| {
            kind(&tree, parent(&tree, *at)) == given.parent && kind(&tree, *at) == given.child
        })
        .nth(given.nth)
        .ok_or(())?;
    assert_eq!(role_of(&tree, found), languages::field_at(&elements, found));
    let name = languages::text_of(
        &symbols,
        tree.source,
        languages::field_name_at(&elements, found),
    )
    .ok_or(())?;
    Ok(Box::leak(name.0.to_owned().into_boxed_str()))
}

/// Every role of the vocabulary has a row of its own.
fn every_role_has_a_row() -> usize {
    fields()
        .filter(|field| {
            let name = super::service::field_name(*field).0;
            !CASES
                .iter()
                .any(|case| matches!(case.expect, Expect::Value(wanted) if wanted == name))
        })
        .count()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    assert_eq!(every_role_has_a_row(), 0, "{:?}", Field::None);
}
