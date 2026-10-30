// ignore: idiomatic.arch/file-length - one row for each role of the vocabulary, the check that every role has one, and go/ast's roles of every pinned file: the table is the proof, and splitting it would split that check
//! The `roles` service's cases: the role of an edge of the tree under its parent, one row for each role of the vocabulary, on source
//! read through the parse; and, for every file the fixtures pin, the role of every element is the field of `go/ast` that holds it, as
//! `tests/fixtures/standard/dump.go` reads it with `reflect` and `output.txt` pins it.

use std::sync::Arc;

use super::service::{field_name, field_of, fields, role_of};
use super::types::AstField;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind, kind_name, parent};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::syntax::factory::tree_sitter_go;
use crate::outbound::syntax::fixtures::library::{pinned, roles};
use ryo::languages::{self, Position, Tree};

/// What a row is given: the nth edge, in source order, from a node of a kind to an element of a kind, or every pinned file.
#[derive(Clone, Copy)]
enum Given {
    /// An edge of the tree of some source.
    Edge {
        /// The source.
        source: &'static str,
        /// The kind of the parent.
        parent: Node,
        /// The kind of the child.
        child: Kind,
        /// Which of the edges of those kinds, from zero.
        nth: usize,
    },
    /// Every relation's input, every file of the corpus and every file of the suite that `go/parser` reads.
    Pinned,
}

/// What a row is given.
type Input = Given;
/// The name of the role the edge plays, or the pinned files whose roles differ from `go/ast`'s, joined with ` | `.
type Value = &'static str;
type Failure = ();

const fn edge(source: &'static str, parent: Node, child: Kind, nth: usize) -> Given {
    Given::Edge {
        source,
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
                "package p\nvar a, b int\n",
                Node::ValueSpec,
                Kind::Token(Token::Comma),
                0,
            )
        },
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("args_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = f(1)\n",
                Node::CallExpr,
                Kind::Node(Node::BasicLit),
                0,
            )
        },
        expect: Expect::Value("args"),
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
        name: CaseName("elt_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ []int\n",
                Node::ArrayType,
                Kind::Node(Node::Ident),
                0,
            )
        },
        expect: Expect::Value("elt"),
    },
    Case {
        name: CaseName("elts_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = []int{1}\n",
                Node::CompositeLit,
                Kind::Node(Node::BasicLit),
                0,
            )
        },
        expect: Expect::Value("elts"),
    },
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
        name: CaseName("high_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = s[1:2]\n",
                Node::SliceExpr,
                Kind::Node(Node::BasicLit),
                1,
            )
        },
        expect: Expect::Value("high"),
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
        name: CaseName("index_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = s[1]\n",
                Node::IndexExpr,
                Kind::Node(Node::BasicLit),
                0,
            )
        },
        expect: Expect::Value("index"),
    },
    Case {
        name: CaseName("indices_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = f[int, string]\n",
                Node::IndexListExpr,
                Kind::Node(Node::Ident),
                1,
            )
        },
        expect: Expect::Value("indices"),
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
        name: CaseName("key_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ map[string]int\n",
                Node::MapType,
                Kind::Node(Node::Ident),
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
                "package p\nfunc f() {\nL:\n\tfor {\n\t\tbreak L\n\t}\n}\n",
                Node::BranchStmt,
                Kind::Node(Node::Ident),
                0,
            )
        },
        expect: Expect::Value("label"),
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
        name: CaseName("len_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ [2]int\n",
                Node::ArrayType,
                Kind::Node(Node::BasicLit),
                0,
            )
        },
        expect: Expect::Value("len"),
    },
    Case {
        name: CaseName("lhs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() { x := 1; _ = x }\n",
                Node::AssignStmt,
                Kind::Node(Node::Ident),
                0,
            )
        },
        expect: Expect::Value("lhs"),
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
        name: CaseName("low_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = s[1:2]\n",
                Node::SliceExpr,
                Kind::Node(Node::BasicLit),
                0,
            )
        },
        expect: Expect::Value("low"),
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
        name: CaseName("max_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = s[1:2:3]\n",
                Node::SliceExpr,
                Kind::Node(Node::BasicLit),
                2,
            )
        },
        expect: Expect::Value("max"),
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
        name: CaseName("rhs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nfunc f() { x := 1; _ = x }\n",
                Node::AssignStmt,
                Kind::Node(Node::BasicLit),
                0,
            )
        },
        expect: Expect::Value("rhs"),
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
        name: CaseName("sel_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = a.b\n",
                Node::SelectorExpr,
                Kind::Node(Node::Ident),
                1,
            )
        },
        expect: Expect::Value("sel"),
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
    Case {
        name: CaseName("x_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = 1 + 2\n",
                Node::BinaryExpr,
                Kind::Node(Node::BasicLit),
                0,
            )
        },
        expect: Expect::Value("x"),
    },
    Case {
        name: CaseName("y_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "package p\nvar _ = 1 + 2\n",
                Node::BinaryExpr,
                Kind::Node(Node::BasicLit),
                1,
            )
        },
        expect: Expect::Value("y"),
    },
    Case {
        name: CaseName("every_pinned_file_gives_each_element_the_field_of_go_ast_that_holds_it"),
        word: Word::Succeeds,
        input: || Given::Pinned,
        expect: Expect::Value(""),
    },
];

/// The table the parse builds of `source`, roles asked for.
fn table(source: &[u8]) -> Result<(languages::Elements, languages::Symbols), Failure> {
    let language = ParseContract::new(
        Arc::new(tree_sitter_go()),
        Arc::new(crate::outbound::notes::factory::postcard()),
    );
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    languages::contract::build(
        &language,
        languages::ParseRequest {
            path: languages::SourcePath("p.go"),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(source),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
    )
    .map_err(drop)?;
    Ok((elements, symbols))
}

/// The roles our tree of `source` gives, as the dump writes `go/ast`'s: `start end N|T kind field`, an edge with no role left out.
fn ours(source: &[u8]) -> Result<Vec<String>, Failure> {
    let (elements, _) = table(source)?;
    let tree = Tree {
        elements: &elements,
        source: languages::ByteSlice(source),
        file: languages::FileId(0),
    };
    let mut lines: Vec<String> = languages::positions(&elements)
        .filter(|at| field_of(&tree, *at) != AstField::None)
        .map(|at| {
            format!(
                "{} {} {} {} {}",
                languages::start_at(&elements, at).0,
                languages::end_at(&elements, at).0,
                if matches!(kind(&tree, at), Kind::Node(_)) {
                    "N"
                } else {
                    "T"
                },
                kind_name(kind(&tree, at)).0,
                field_name(field_of(&tree, at)).0,
            )
        })
        .collect();
    lines.sort();
    Ok(lines)
}

/// The pinned files whose roles differ from `go/ast`'s, by path below `tests/fixtures`, each with its first difference.
fn differing() -> Result<Vec<String>, Failure> {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let files = pinned(&fixtures);
    if files.is_empty() {
        return Err(());
    }
    let mut found = Vec::new();
    for path in files {
        let bytes = std::fs::read(&path).map_err(drop)?;
        let Some(mut theirs) = roles(&String::from_utf8_lossy(&bytes)) else {
            continue;
        };
        theirs.sort_unstable();
        let ours = ours(&bytes)?;
        let missing = theirs
            .iter()
            .find(|line| !ours.iter().any(|each| each == *line));
        let added = ours.iter().find(|line| !theirs.contains(&line.as_str()));
        if missing.is_some() || added.is_some() {
            found.push(format!(
                "{}: go/ast {:?}, ours {:?}",
                path.strip_prefix(&fixtures).unwrap_or(&path).display(),
                missing,
                added
            ));
        }
    }
    Ok(found)
}

/// The role the edge plays in the table the parse builds of its source, or the pinned files whose roles differ.
fn subject(given: Input) -> Result<Value, Failure> {
    let Given::Edge {
        source,
        parent: holder,
        child,
        nth,
    } = given
    else {
        return Ok(Box::leak(differing()?.join(" | ").into_boxed_str()));
    };
    let (elements, symbols) = table(source.as_bytes())?;
    let tree = Tree {
        elements: &elements,
        source: languages::ByteSlice(source.as_bytes()),
        file: languages::FileId(0),
    };
    let found = languages::positions(&elements)
        .filter(|at| *at != Position(0))
        .filter(|at| {
            kind(&tree, parent(&tree, *at)) == Kind::Node(holder) && kind(&tree, *at) == child
        })
        .nth(nth)
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

/// The roles of the vocabulary with no row of their own.
fn roles_without_a_row() -> Vec<AstField> {
    fields()
        .filter(|field| {
            let name = field_name(*field).0;
            !CASES
                .iter()
                .any(|case| matches!(case.expect, Expect::Value(wanted) if wanted == name))
        })
        .collect()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    assert_eq!(roles_without_a_row(), Vec::<AstField>::new());
}
