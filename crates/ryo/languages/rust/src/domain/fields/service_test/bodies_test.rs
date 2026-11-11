//! The `fields` service's cases for bodies: the role of an edge of the tree under its parent.

use super::super::fixtures::{Edge, edge};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;

/// What a row is given.
type Input = Edge;
/// The name of the role the edge plays.
type Value = &'static str;
type Failure = ();

/// Reads one row's input through the concept fixtures' shared reading.
fn subject(input: Input) -> Result<Value, Failure> {
    super::super::fixtures::subject(input)
}

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("let_else_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { let a = b else { return }; }",
                Kind::Node(Node::LetStmt),
                Kind::Node(Node::LetElse),
                0,
            )
        },
        expect: Expect::Value("let-else"),
    },
    Case {
        name: CaseName("tail_expr_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { 1 }",
                Kind::Node(Node::StmtList),
                Kind::Node(Node::Literal),
                0,
            )
        },
        expect: Expect::Value("tail-expr"),
    },
    Case {
        name: CaseName("condition_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { if a {} }",
                Kind::Node(Node::IfExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("condition"),
    },
    Case {
        name: CaseName("then_branch_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { if a {} }",
                Kind::Node(Node::IfExpr),
                Kind::Node(Node::BlockExpr),
                0,
            )
        },
        expect: Expect::Value("then-branch"),
    },
    Case {
        name: CaseName("else_branch_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { if a {} else {} }",
                Kind::Node(Node::IfExpr),
                Kind::Node(Node::BlockExpr),
                1,
            )
        },
        expect: Expect::Value("else-branch"),
    },
    Case {
        name: CaseName("loop_body_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { loop {} }",
                Kind::Node(Node::LoopExpr),
                Kind::Node(Node::BlockExpr),
                0,
            )
        },
        expect: Expect::Value("loop-body"),
    },
    Case {
        name: CaseName("iterable_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { for a in b {} }",
                Kind::Node(Node::ForExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("iterable"),
    },
    Case {
        name: CaseName("initializer_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { let a = 1; }",
                Kind::Node(Node::LetStmt),
                Kind::Node(Node::Literal),
                0,
            )
        },
        expect: Expect::Value("initializer"),
    },
    Case {
        name: CaseName("guard_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { match x { _ if y => 1 } }",
                Kind::Node(Node::MatchArm),
                Kind::Node(Node::MatchGuard),
                0,
            )
        },
        expect: Expect::Value("guard"),
    },
    Case {
        name: CaseName("body_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::BlockExpr),
                0,
            )
        },
        expect: Expect::Value("body"),
    },
    Case {
        name: CaseName("default_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f<T = u8>() {}",
                Kind::Node(Node::TypeParam),
                Kind::Node(Node::PathType),
                0,
            )
        },
        expect: Expect::Value("default-type"),
    },
    Case {
        name: CaseName("default_val_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f<const N: usize = 1>() {}",
                Kind::Node(Node::ConstParam),
                Kind::Node(Node::ConstArg),
                0,
            )
        },
        expect: Expect::Value("default-val"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
