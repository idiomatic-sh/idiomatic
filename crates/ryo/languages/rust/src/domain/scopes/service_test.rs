//! The `scopes` service's cases, on trees written by hand: each answers what the service said, in full.

use crate::domain::nodes::types::Node;
use ryo::languages::{self, Position, Tree};

use super::service::{enclosing_function, is_nameless_function};
use super::types::Namelessness;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::kind;
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::fixtures::script::table_of;
use crate::outbound::syntax::fixtures::trees::tree_of;

/// What a row asks of the first element of a kind.
#[derive(Clone, Copy)]
enum Question {
    /// The kind of the function that encloses it.
    Enclosing(Kind),
    /// Whether it is a function that writes no name.
    Nameless(Kind),
}

/// The source, and what is asked of it.
type Input = (&'static str, Question);

/// What the service answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    /// The enclosing function starts at this place of the table.
    Enclosed(u32),
    /// Nothing encloses it.
    Free,
    /// Whether a function writes no name.
    Namelessness(Namelessness),
}
type Value = Answer;
type Failure = ();

fn subject((source, question): Input) -> Result<Value, Failure> {
    let (elements, _symbols, _) =
        table_of(&tree_of(source).expect("a tree is written for the source"));
    let tree = Tree {
        elements: &elements,
        source: languages::ByteSlice(source.as_bytes()),
        file: languages::FileId(0),
    };
    let first = |wanted: Kind| -> Result<Position, Failure> {
        languages::positions(tree.elements)
            .find(|at| kind(&tree, *at) == wanted)
            .ok_or(())
    };
    Ok(match question {
        Question::Enclosing(of) => enclosing_function(&tree, first(of)?)
            .map_or(Answer::Free, |function| Answer::Enclosed(function.0)),
        Question::Nameless(of) => Answer::Namelessness(is_nameless_function(&tree, first(of)?)),
    })
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_statement_is_enclosed_by_its_function"),
        word: Word::Succeeds,
        input: || {
            (
                "fn f() { let x = 1; }",
                Question::Enclosing(Kind::Node(Node::LetStmt)),
            )
        },
        expect: Expect::Value(Answer::Enclosed(1)),
    },
    Case {
        name: CaseName("a_function_is_not_enclosed_by_itself"),
        word: Word::Succeeds,
        input: || ("fn f() {}", Question::Enclosing(Kind::Node(Node::Fn))),
        expect: Expect::Value(Answer::Free),
    },
    Case {
        name: CaseName("a_function_with_a_name_is_not_nameless"),
        word: Word::Succeeds,
        input: || ("fn f() {}", Question::Nameless(Kind::Node(Node::Fn))),
        expect: Expect::Value(Answer::Namelessness(Namelessness::Named)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
