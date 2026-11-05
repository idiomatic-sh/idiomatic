//! The `references` service's cases, on trees written by hand: each answers what the service said, in full.

use crate::domain::nodes::types::Node;
use ryo::languages::{self, Position, Tree};

use super::service::{is_single, is_written_use};
use super::types::{Singleness, Usage};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::kind;
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::fixtures::script::table_of;
use crate::outbound::syntax::fixtures::trees::tree_of;

/// What a row asks of the first path of a source.
#[derive(Clone, Copy)]
enum Question {
    /// Whether it writes exactly one name.
    Single,
    /// Whether it is a use of a name.
    Written,
}

/// The source, and what is asked of its first path.
type Input = (&'static str, Question);

/// What the service answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    /// How many names the path writes.
    Singleness(Singleness),
    /// Whether it is a use of a name.
    Usage(Usage),
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
    let path: Position = languages::positions(tree.elements)
        .find(|at| kind(&tree, *at) == Kind::Node(Node::Path))
        .ok_or(())?;
    Ok(match question {
        Question::Single => Answer::Singleness(is_single(&tree, path)),
        Question::Written => Answer::Usage(is_written_use(&tree, path)),
    })
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_path_of_one_name_is_single"),
        word: Word::Succeeds,
        input: || ("fn f() { g(); }", Question::Single),
        expect: Expect::Value(Answer::Singleness(Singleness::Single)),
    },
    Case {
        name: CaseName("a_path_of_two_names_is_not_single"),
        word: Word::Succeeds,
        input: || ("fn f() { a::g(); }", Question::Single),
        expect: Expect::Value(Answer::Singleness(Singleness::Several)),
    },
    Case {
        name: CaseName("a_path_in_a_call_is_a_written_use"),
        word: Word::Succeeds,
        input: || ("fn f() { g(); }", Question::Written),
        expect: Expect::Value(Answer::Usage(Usage::Written)),
    },
    Case {
        name: CaseName("a_path_in_a_use_is_not"),
        word: Word::Succeeds,
        input: || ("use a::b;", Question::Written),
        expect: Expect::Value(Answer::Usage(Usage::Skipped)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
