//! The `roles` service's cases for declarations: the role of an edge of the tree under its parent.

use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::cases::{Case, CaseName, Expect, Word, run};
use super::fixtures::{Given, edge, subject};





/// What a row is given.
type Input = Given;
/// The name of the role the edge plays, or the pinned files whose roles differ from `go/ast`'s, joined with ` | `.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("fields_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\ntype T struct{}\n",
                Node::StructType,
                Kind::Node(Node::FieldList),
                0,
            )
        },
        expect: Expect::Value("fields"),
    },
    Case {
        name: CaseName("interface_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\ntype I interface{}\n",
                Node::InterfaceType,
                Kind::Token(Token::Interface),
                0,
            )
        },
        expect: Expect::Value("interface"),
    },
    Case {
        name: CaseName("map_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ map[string]int\n",
                Node::MapType,
                Kind::Token(Token::Map),
                0,
            )
        },
        expect: Expect::Value("map"),
    },
    Case {
        name: CaseName("methods_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\ntype I interface{ M() }\n",
                Node::InterfaceType,
                Kind::Node(Node::FieldList),
                0,
            )
        },
        expect: Expect::Value("methods"),
    },
    Case {
        name: CaseName("name_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {}\n",
                Node::FuncType,
                Kind::Node(Node::Ident),
                0,
            )
        },
        expect: Expect::Value("name"),
    },
    Case {
        name: CaseName("names_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f(a, b int) {}\n",
                Node::Field,
                Kind::Node(Node::Ident),
                1,
            )
        },
        expect: Expect::Value("names"),
    },
    Case {
        name: CaseName("package_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || edge("package p\n", Node::File, Kind::Token(Token::Package), 0),
        expect: Expect::Value("package"),
    },
    Case {
        name: CaseName("params_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc (r T) f(a int) (b int) {}\n",
                Node::FuncType,
                Kind::Node(Node::FieldList),
                1,
            )
        },
        expect: Expect::Value("params"),
    },
    Case {
        name: CaseName("path_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nimport f \"fmt\"\n",
                Node::ImportSpec,
                Kind::Node(Node::BasicLit),
                0,
            )
        },
        expect: Expect::Value("path"),
    },
    Case {
        name: CaseName("recv_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc (r T) f(a int) (b int) {}\n",
                Node::FuncType,
                Kind::Node(Node::FieldList),
                0,
            )
        },
        expect: Expect::Value("recv"),
    },
    Case {
        name: CaseName("results_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc (r T) f(a int) (b int) {}\n",
                Node::FuncType,
                Kind::Node(Node::FieldList),
                2,
            )
        },
        expect: Expect::Value("results"),
    },
    Case {
        name: CaseName("specs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar x int\n",
                Node::GenDecl,
                Kind::Node(Node::ValueSpec),
                0,
            )
        },
        expect: Expect::Value("specs"),
    },
    Case {
        name: CaseName("struct_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\ntype T struct{}\n",
                Node::StructType,
                Kind::Token(Token::Struct),
                0,
            )
        },
        expect: Expect::Value("struct"),
    },
    Case {
        name: CaseName("tag_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\ntype T struct {\n\tx int `json:\"x\"`\n}\n",
                Node::Field,
                Kind::Node(Node::BasicLit),
                0,
            )
        },
        expect: Expect::Value("tag"),
    },
    Case {
        name: CaseName("type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f(a int) {}\n",
                Node::Field,
                Kind::Node(Node::Ident),
                1,
            )
        },
        expect: Expect::Value("type"),
    },
    Case {
        name: CaseName("type_params_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f[T any]() {}\n",
                Node::FuncType,
                Kind::Node(Node::FieldList),
                0,
            )
        },
        expect: Expect::Value("type-params"),
    },
    Case {
        name: CaseName("value_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = 1\n",
                Node::BasicLit,
                Kind::Token(Token::Int),
                0,
            )
        },
        expect: Expect::Value("value"),
    },
    Case {
        name: CaseName("values_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar x = 1\n",
                Node::ValueSpec,
                Kind::Node(Node::BasicLit),
                0,
            )
        },
        expect: Expect::Value("values"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
