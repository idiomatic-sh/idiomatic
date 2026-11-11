//! The `roles` service's cases for statements: the role of an edge of the tree under its parent.

use super::super::fixtures::{Given, edge};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// What a row is given.
type Input = Given;
/// The name of the role the edge plays, or the pinned files whose roles differ from `go/ast`'s, joined with ` | `.
type Value = &'static str;
type Failure = ();

/// Reads one row's input through the concept fixtures' shared reading.
fn subject(input: Input) -> Result<Value, Failure> {
    super::super::fixtures::subject(input)
}

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("body_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {}\n",
                Node::FuncDecl,
                Kind::Node(Node::BlockStmt),
                0,
            )
        },
        expect: Expect::Value("body"),
    },
    Case {
        name: CaseName("call_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() { go f() }\n",
                Node::GoStmt,
                Kind::Node(Node::CallExpr),
                0,
            )
        },
        expect: Expect::Value("call"),
    },
    Case {
        name: CaseName("case_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f(x int) {\n\tswitch x {\n\tcase 1:\n\t}\n}\n",
                Node::CaseClause,
                Kind::Token(Token::Case),
                0,
            )
        },
        expect: Expect::Value("case"),
    },
    Case {
        name: CaseName("chan_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f(c chan int) { c <- 1 }\n",
                Node::SendStmt,
                Kind::Node(Node::Ident),
                0,
            )
        },
        expect: Expect::Value("chan"),
    },
    Case {
        name: CaseName("cond_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f(x bool) {\n\tif x {\n\t}\n}\n",
                Node::IfStmt,
                Kind::Node(Node::Ident),
                0,
            )
        },
        expect: Expect::Value("cond"),
    },
    Case {
        name: CaseName("decl_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() { var x int; _ = x }\n",
                Node::DeclStmt,
                Kind::Node(Node::GenDecl),
                0,
            )
        },
        expect: Expect::Value("decl"),
    },
    Case {
        name: CaseName("decls_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {}\n",
                Node::File,
                Kind::Node(Node::FuncDecl),
                0,
            )
        },
        expect: Expect::Value("decls"),
    },
    Case {
        name: CaseName("defer_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() { defer f() }\n",
                Node::DeferStmt,
                Kind::Token(Token::Defer),
                0,
            )
        },
        expect: Expect::Value("defer"),
    },
    Case {
        name: CaseName("else_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f(x bool) {\n\tif x {\n\t} else {\n\t}\n}\n",
                Node::IfStmt,
                Kind::Node(Node::BlockStmt),
                1,
            )
        },
        expect: Expect::Value("else"),
    },
    Case {
        name: CaseName("for_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {\n\tfor {\n\t}\n}\n",
                Node::ForStmt,
                Kind::Token(Token::For),
                0,
            )
        },
        expect: Expect::Value("for"),
    },
    Case {
        name: CaseName("fun_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = f(1)\n",
                Node::CallExpr,
                Kind::Node(Node::Ident),
                0,
            )
        },
        expect: Expect::Value("fun"),
    },
    Case {
        name: CaseName("func_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {}\n",
                Node::FuncType,
                Kind::Token(Token::Func),
                0,
            )
        },
        expect: Expect::Value("func"),
    },
    Case {
        name: CaseName("go_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() { go f() }\n",
                Node::GoStmt,
                Kind::Token(Token::Go),
                0,
            )
        },
        expect: Expect::Value("go"),
    },
    Case {
        name: CaseName("if_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f(x bool) {\n\tif x {\n\t}\n}\n",
                Node::IfStmt,
                Kind::Token(Token::If),
                0,
            )
        },
        expect: Expect::Value("if"),
    },
    Case {
        name: CaseName("init_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {\n\tif x := 1; x > 0 {\n\t}\n}\n",
                Node::IfStmt,
                Kind::Node(Node::AssignStmt),
                0,
            )
        },
        expect: Expect::Value("init"),
    },
    Case {
        name: CaseName("label_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {\nL:\n\tfor {\n\t\tbreak L\n\t}\n}\n",
                Node::BranchStmt,
                Kind::Node(Node::Ident),
                0,
            )
        },
        expect: Expect::Value("label"),
    },
    Case {
        name: CaseName("list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() { f() }\n",
                Node::BlockStmt,
                Kind::Node(Node::ExprStmt),
                0,
            )
        },
        expect: Expect::Value("list"),
    },
    Case {
        name: CaseName("post_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {\n\tfor i := 0; i < 1; i++ {\n\t}\n}\n",
                Node::ForStmt,
                Kind::Node(Node::IncDecStmt),
                0,
            )
        },
        expect: Expect::Value("post"),
    },
    Case {
        name: CaseName("range_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f(s []int) {\n\tfor range s {\n\t}\n}\n",
                Node::RangeStmt,
                Kind::Token(Token::Range),
                0,
            )
        },
        expect: Expect::Value("range"),
    },
    Case {
        name: CaseName("return_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() { return }\n",
                Node::ReturnStmt,
                Kind::Token(Token::Return),
                0,
            )
        },
        expect: Expect::Value("return"),
    },
    Case {
        name: CaseName("select_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {\n\tselect {\n\t}\n}\n",
                Node::SelectStmt,
                Kind::Token(Token::Select),
                0,
            )
        },
        expect: Expect::Value("select"),
    },
    Case {
        name: CaseName("stmt_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {\nL:\n\tfor {\n\t\tbreak L\n\t}\n}\n",
                Node::LabeledStmt,
                Kind::Node(Node::ForStmt),
                0,
            )
        },
        expect: Expect::Value("stmt"),
    },
    Case {
        name: CaseName("switch_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() {\n\tswitch {\n\t}\n}\n",
                Node::SwitchStmt,
                Kind::Token(Token::Switch),
                0,
            )
        },
        expect: Expect::Value("switch"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
