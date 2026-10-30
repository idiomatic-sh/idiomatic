//! The `bindings` service's cases, on trees written by hand: each answers what the service said, in full.

use crate::domain::nodes::types::Node;
use ryo::languages::{self, Position, Tree};

use super::service::{bound_name, in_let_pattern, looks_like_constant};
use super::types::{Constness, LetPattern};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::{end, kind, start};
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::fixtures::script::table_of;
use crate::outbound::syntax::fixtures::trees::tree_of;

/// What a row asks of the first element of a kind in the tree of a source.
#[derive(Clone, Copy)]
enum Question {
    /// The name the first element of this kind binds.
    Bound(Kind),
    /// Whether the name it binds is nested in a destructuring `let`.
    LetPattern(Kind),
    /// Whether the name it binds starts as a constant is written.
    Constant(Kind),
}

/// The source, and what is asked of it.
type Input = (&'static str, Question);

/// What the service answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    /// A name, by its text.
    Name(&'static str),
    /// No name.
    Nothing,
    /// Whether a name is nested in a destructuring `let`.
    LetPattern(LetPattern),
    /// Whether a name starts as a constant is written.
    Constness(Constness),
}
type Value = Answer;
type Failure = ();

/// The text of the element at `at`.
fn text(tree: &Tree<'_>, at: Position) -> &'static str {
    let (from, to) = (
        usize::try_from(start(tree, at).0).unwrap_or(0),
        usize::try_from(end(tree, at).0).unwrap_or(0),
    );
    Box::leak(
        String::from_utf8_lossy(tree.source.0.get(from..to).unwrap_or_default())
            .into_owned()
            .into_boxed_str(),
    )
}

fn subject((source, question): Input) -> Result<Value, Failure> {
    let (elements, _symbols, _) =
        table_of(&tree_of(source).expect("a tree is written for the source"));
    let tree = Tree {
        elements: &elements,
        source: languages::ByteSlice(source.as_bytes()),
        file: languages::FileId(0),
    };
    let first = |wanted: Kind| {
        languages::positions(tree.elements)
            .find(|at| kind(&tree, *at) == wanted)
            .ok_or(())
    };
    Ok(match question {
        Question::Bound(owner) => bound_name(&tree, first(owner)?)
            .map_or(Answer::Nothing, |name| Answer::Name(text(&tree, name))),
        Question::LetPattern(owner) => Answer::LetPattern(in_let_pattern(
            &tree,
            bound_name(&tree, first(owner)?).ok_or(())?,
        )),
        Question::Constant(owner) => Answer::Constness(looks_like_constant(
            &tree,
            bound_name(&tree, first(owner)?).ok_or(())?,
        )),
    })
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_parameter_that_is_one_name_binds_it"),
        word: Word::Succeeds,
        input: || ("fn f(a: u32) {}", Question::Bound(Kind::Node(Node::Param))),
        expect: Expect::Value(Answer::Name("a")),
    },
    Case {
        name: CaseName("a_destructuring_parameter_binds_no_name"),
        word: Word::Succeeds,
        input: || {
            (
                "fn f((a, b): (u32, u32)) {}",
                Question::Bound(Kind::Node(Node::Param)),
            )
        },
        expect: Expect::Value(Answer::Nothing),
    },
    Case {
        name: CaseName("a_let_that_is_one_name_binds_it"),
        word: Word::Succeeds,
        input: || {
            (
                "fn f() { let x = 1; }",
                Question::Bound(Kind::Node(Node::LetStmt)),
            )
        },
        expect: Expect::Value(Answer::Name("x")),
    },
    Case {
        name: CaseName("the_name_of_a_let_is_in_its_pattern"),
        word: Word::Succeeds,
        input: || {
            (
                "fn f() { let x = 1; }",
                Question::LetPattern(Kind::Node(Node::LetStmt)),
            )
        },
        expect: Expect::Value(Answer::LetPattern(LetPattern::In)),
    },
    Case {
        name: CaseName("an_upper_case_name_looks_like_a_constant"),
        word: Word::Succeeds,
        input: || {
            (
                "fn f() { let MAX = 1; }",
                Question::Constant(Kind::Node(Node::LetStmt)),
            )
        },
        expect: Expect::Value(Answer::Constness(Constness::Constant)),
    },
    Case {
        name: CaseName("a_lower_case_name_does_not"),
        word: Word::Succeeds,
        input: || {
            (
                "fn f() { let x = 1; }",
                Question::Constant(Kind::Node(Node::LetStmt)),
            )
        },
        expect: Expect::Value(Answer::Constness(Constness::NotConstant)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
