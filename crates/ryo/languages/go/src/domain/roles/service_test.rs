//! The `roles` service's cases: the role of an edge of the tree under its parent, one row for each role of the vocabulary, on source
//! read through the parse; and, for every file the fixtures pin, the role of every element is the field of `go/ast` that holds it, as
//! `tests/fixtures/standard/dump.go` reads it with `reflect` and `output.txt` pins it.

use super::fixtures::{Given, edge};
use super::service::{field_name, fields};
use super::types::AstField;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;

#[cfg(test)]
mod declarations_test;
#[cfg(test)]
mod pinned_test;
#[cfg(test)]
mod punctuation_test;
#[cfg(test)]
mod statements_test;

/// What a row is given.
type Input = Given;
/// The name of the role the edge plays, or the pinned files whose roles differ from `go/ast`'s, joined with ` | `.
type Value = &'static str;
type Failure = ();

/// Reads one row's input through the concept fixtures' shared reading.
fn subject(input: Input) -> Result<Value, Failure> {
    super::fixtures::subject(input)
}

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
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
];

/// The rows of every table of the part's cases.
fn all_cases() -> impl Iterator<Item = &'static Case<Input, Value, Failure>> {
    [
        CASES,
        punctuation_test::CASES,
        statements_test::CASES,
        declarations_test::CASES,
        pinned_test::CASES,
    ]
    .into_iter()
    .flat_map(|table| table.iter())
}

/// The roles of the vocabulary with no row of their own.
fn roles_without_a_row() -> Vec<AstField> {
    fields()
        .filter(|field| {
            let name = field_name(*field).0;
            !all_cases().any(|case| matches!(case.expect, Expect::Value(wanted) if wanted == name))
        })
        .collect()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    assert_eq!(roles_without_a_row(), Vec::<AstField>::new());
}
