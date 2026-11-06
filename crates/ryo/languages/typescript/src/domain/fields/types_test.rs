//! The `fields` service's cases for types: the role of an edge of the tree under its parent.

use super::fixtures::{Dialect, Edge, edge, subject};
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
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
