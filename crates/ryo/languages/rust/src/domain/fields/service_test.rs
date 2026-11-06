//! The `fields` service's cases: the role of an edge of the tree under its parent, one row for each role of the vocabulary, on source
//! read through the parse. The roles are those the labels of `rust.ungram` give; the adapter of the syntax port holds them to the typed
//! accessors of the library.

use super::fixtures::{Edge, edge, subject};
use super::service::fields;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::labels::types::Field;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// What a row is given.
type Input = Edge;
/// The name of the role the edge plays.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("where_clause_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() where T: A {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::WhereClause),
                0,
            )
        },
        expect: Expect::Value("where-clause"),
    },
    Case {
        name: CaseName("ret_type_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() -> u8 {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::RetType),
                0,
            )
        },
        expect: Expect::Value("ret-type"),
    },
    Case {
        name: CaseName("use_tree_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "use a;",
                Kind::Node(Node::Use),
                Kind::Node(Node::UseTree),
                0,
            )
        },
        expect: Expect::Value("use-tree"),
    },
    Case {
        name: CaseName("expr_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { x.y; }",
                Kind::Node(Node::FieldExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("expr"),
    },
    Case {
        name: CaseName("pat_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f(a: u8) {}",
                Kind::Node(Node::Param),
                Kind::Node(Node::IdentPat),
                0,
            )
        },
        expect: Expect::Value("pat"),
    },
    Case {
        name: CaseName("ty_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f(a: u8) {}",
                Kind::Node(Node::Param),
                Kind::Node(Node::PathType),
                0,
            )
        },
        expect: Expect::Value("ty"),
    },
    Case {
        name: CaseName("receiver_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a.b(); }",
                Kind::Node(Node::MethodCallExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("receiver"),
    },
    Case {
        name: CaseName("lhs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a + b; }",
                Kind::Node(Node::BinExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("lhs"),
    },
    Case {
        name: CaseName("rhs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a + b; }",
                Kind::Node(Node::BinExpr),
                Kind::Node(Node::PathExpr),
                1,
            )
        },
        expect: Expect::Value("rhs"),
    },
    Case {
        name: CaseName("op_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a + b; }",
                Kind::Node(Node::BinExpr),
                Kind::Token(Token::Plus),
                0,
            )
        },
        expect: Expect::Value("op"),
    },
    Case {
        name: CaseName("base_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a[b]; }",
                Kind::Node(Node::IndexExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("base"),
    },
    Case {
        name: CaseName("index_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a[b]; }",
                Kind::Node(Node::IndexExpr),
                Kind::Node(Node::PathExpr),
                1,
            )
        },
        expect: Expect::Value("index"),
    },
    Case {
        name: CaseName("start_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a..b; }",
                Kind::Node(Node::RangeExpr),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("start"),
    },
    Case {
        name: CaseName("end_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a..b; }",
                Kind::Node(Node::RangeExpr),
                Kind::Node(Node::PathExpr),
                1,
            )
        },
        expect: Expect::Value("end"),
    },
    Case {
        name: CaseName("spread_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { S { ..a }; }",
                Kind::Node(Node::RecordExprFieldList),
                Kind::Node(Node::PathExpr),
                0,
            )
        },
        expect: Expect::Value("spread"),
    },
];

/// The rows of every table of the part's cases.
fn all_cases() -> impl Iterator<Item = &'static Case<Input, Value, Failure>> {
    [
        CASES,
        super::identity_test::CASES,
        super::lists_test::CASES,
        super::bodies_test::CASES,
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
