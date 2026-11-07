//! The `roles` service's cases for punctuation: the role of an edge of the tree under its parent.

use super::fixtures::{Given, edge, subject};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// What a row is given.
type Input = Given;
/// The name of the role the edge plays, or the pinned files whose roles differ from `go/ast`'s, joined with ` | `.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("none_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar a, b int\n",
                Node::ValueSpec,
                Kind::Token(Token::Comma),
                0,
            )
        },
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("arrow_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f(c chan<- int) { c <- 1 }\n",
                Node::SendStmt,
                Kind::Token(Token::Arrow),
                0,
            )
        },
        expect: Expect::Value("arrow"),
    },
    Case {
        name: CaseName("assign_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\ntype A = int\n",
                Node::TypeSpec,
                Kind::Token(Token::Assign),
                0,
            )
        },
        expect: Expect::Value("assign"),
    },
    Case {
        name: CaseName("begin_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar c <-chan int\n",
                Node::ChanType,
                Kind::Token(Token::Arrow),
                0,
            )
        },
        expect: Expect::Value("begin"),
    },
    Case {
        name: CaseName("closing_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {}\n",
                Node::FieldList,
                Kind::Token(Token::Rparen),
                0,
            )
        },
        expect: Expect::Value("closing"),
    },
    Case {
        name: CaseName("colon_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = map[int]int{1: 2}\n",
                Node::KeyValueExpr,
                Kind::Token(Token::Colon),
                0,
            )
        },
        expect: Expect::Value("colon"),
    },
    Case {
        name: CaseName("comm_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f(c chan int) {\n\tselect {\n\tcase <-c:\n\t}\n}\n",
                Node::CommClause,
                Kind::Node(Node::ExprStmt),
                0,
            )
        },
        expect: Expect::Value("comm"),
    },
    Case {
        name: CaseName("ellipsis_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f(x ...int) {}\n",
                Node::Ellipsis,
                Kind::Token(Token::Ellipsis),
                0,
            )
        },
        expect: Expect::Value("ellipsis"),
    },
    Case {
        name: CaseName("lbrace_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {}\n",
                Node::BlockStmt,
                Kind::Token(Token::Lbrace),
                0,
            )
        },
        expect: Expect::Value("lbrace"),
    },
    Case {
        name: CaseName("lbrack_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = s[1]\n",
                Node::IndexExpr,
                Kind::Token(Token::Lbrack),
                0,
            )
        },
        expect: Expect::Value("lbrack"),
    },
    Case {
        name: CaseName("lparen_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = f(1)\n",
                Node::CallExpr,
                Kind::Token(Token::Lparen),
                0,
            )
        },
        expect: Expect::Value("lparen"),
    },
    Case {
        name: CaseName("op_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = 1 + 2\n",
                Node::BinaryExpr,
                Kind::Token(Token::Add),
                0,
            )
        },
        expect: Expect::Value("op"),
    },
    Case {
        name: CaseName("opening_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {}\n",
                Node::FieldList,
                Kind::Token(Token::Lparen),
                0,
            )
        },
        expect: Expect::Value("opening"),
    },
    Case {
        name: CaseName("rbrace_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {}\n",
                Node::BlockStmt,
                Kind::Token(Token::Rbrace),
                0,
            )
        },
        expect: Expect::Value("rbrace"),
    },
    Case {
        name: CaseName("rbrack_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = s[1]\n",
                Node::IndexExpr,
                Kind::Token(Token::Rbrack),
                0,
            )
        },
        expect: Expect::Value("rbrack"),
    },
    Case {
        name: CaseName("rparen_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = f(1)\n",
                Node::CallExpr,
                Kind::Token(Token::Rparen),
                0,
            )
        },
        expect: Expect::Value("rparen"),
    },
    Case {
        name: CaseName("semicolon_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {\n\tfor {\n\t\t;\n\t}\n}\n",
                Node::EmptyStmt,
                Kind::Token(Token::Semicolon),
                0,
            )
        },
        expect: Expect::Value("semicolon"),
    },
    Case {
        name: CaseName("star_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ *int\n",
                Node::StarExpr,
                Kind::Token(Token::Mul),
                0,
            )
        },
        expect: Expect::Value("star"),
    },
    Case {
        name: CaseName("tok_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() { x := 1; _ = x }\n",
                Node::AssignStmt,
                Kind::Token(Token::Define),
                0,
            )
        },
        expect: Expect::Value("tok"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
