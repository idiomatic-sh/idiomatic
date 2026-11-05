//! The `assignments` service's cases: the operator of an assignment, and the two sides of it.

use super::service::{left, operator, right};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::fixtures::reading::{asked, of_kind, spans};

/// What a row asks of the first assignment or `range` clause of the source.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// Its operator.
    Operator,
    /// What it assigns to.
    Left,
    /// What it assigns.
    Right,
}

type Input = (&'static str, Node, Question);
/// The answer, as text.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_short_declaration_has_the_define_operator"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nfunc f() { a, b := 1, 2 }\n",
                Node::AssignStmt,
                Question::Operator,
            )
        },
        expect: Expect::Value("Define"),
    },
    Case {
        name: CaseName("an_operator_that_computes_is_an_assignment_operator"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nfunc f() { a += 1 }\n",
                Node::AssignStmt,
                Question::Operator,
            )
        },
        expect: Expect::Value("AddAssign"),
    },
    Case {
        name: CaseName("the_left_side_is_what_is_before_the_operator"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nfunc f() { a, b := 1, 2 }\n",
                Node::AssignStmt,
                Question::Left,
            )
        },
        expect: Expect::Value("a b"),
    },
    Case {
        name: CaseName("the_right_side_is_what_is_after_the_operator"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nfunc f() { a, b := 1, 2 }\n",
                Node::AssignStmt,
                Question::Right,
            )
        },
        expect: Expect::Value("1 2"),
    },
    Case {
        name: CaseName("a_range_clause_assigns_to_what_is_before_its_operator"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nfunc f() { for i, v := range xs {} }\n",
                Node::RangeStmt,
                Question::Left,
            )
        },
        expect: Expect::Value("i v"),
    },
    Case {
        name: CaseName("a_range_clause_with_no_operator_before_its_range_assigns_to_nothing"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nfunc f() { for range xs {} }\n",
                Node::RangeStmt,
                Question::Left,
            )
        },
        expect: Expect::Value(""),
    },
];

fn subject((source, node, question): Input) -> Value {
    let answer = asked(source, |tree| {
        let Some(at) = of_kind(tree, Kind::Node(node)).first().copied() else {
            return String::new();
        };
        let joined = |found: Vec<ryo::languages::Position>| {
            found
                .into_iter()
                .map(|each| spans(tree, each))
                .collect::<Vec<_>>()
                .join(" ")
        };
        match question {
            Question::Operator => {
                operator(tree, at).map_or_else(String::new, |each| format!("{:?}", each.token))
            }
            Question::Left => joined(left(tree, at).collect()),
            Question::Right => joined(right(tree, at).collect()),
        }
    });
    Box::leak(answer.into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
