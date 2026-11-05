//! The `siblings` service's cases: where the second operand of a binary expression stands among its siblings, a comment between them
//! left out.

use super::service::{among, before, first, last, nodes_after, previous};
use super::types::Wanted;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::GoNode;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::fixtures::reading::{asked, of_kind};

/// What a row asks of the operand `y` in `x /* c */ + y`.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// How many nodes come before it.
    NodesBefore,
    /// How many `+` are among its siblings.
    Pluses,
    /// How many nodes come after it.
    NodesAfter,
    /// What is right before it.
    Previous,
    /// Whether it is first, and whether it is last.
    Ends,
}

type Input = Question;
/// The answer, as text.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_node_comes_before_the_second_operand"),
        word: Word::Succeeds,
        input: || Question::NodesBefore,
        expect: Expect::Value("1"),
    },
    Case {
        name: CaseName("the_operator_is_among_the_siblings_once"),
        word: Word::Succeeds,
        input: || Question::Pluses,
        expect: Expect::Value("1"),
    },
    Case {
        name: CaseName("no_node_comes_after_the_second_operand"),
        word: Word::Succeeds,
        input: || Question::NodesAfter,
        expect: Expect::Value("0"),
    },
    Case {
        name: CaseName("the_operator_is_right_before_the_second_operand_white_space_left_out"),
        word: Word::Succeeds,
        input: || Question::Previous,
        expect: Expect::Value("Some(Token(Add))"),
    },
    Case {
        name: CaseName("the_second_operand_is_last_and_not_first_the_comment_left_out"),
        word: Word::Succeeds,
        input: || Question::Ends,
        expect: Expect::Value("Inside Edge"),
    },
];

fn subject(question: Input) -> Result<Value, Failure> {
    let answer = asked("package p\nvar _ = x /* c */ + y\n", |tree| {
        let operation = *of_kind(tree, Kind::Node(GoNode::BinaryExpr)).first()?;
        let operand = ryo::languages::children(tree.elements, operation)
            .filter(|each| {
                crate::domain::syntax::service::kind(tree, *each) == Kind::Node(GoNode::Ident)
            })
            .last()?;
        Some(match question {
            Question::NodesBefore => before(tree, operand, Wanted::Node).0.to_string(),
            Question::Pluses => among(tree, operand, Wanted::Token(Token::Add))
                .0
                .to_string(),
            Question::NodesAfter => nodes_after(tree, operand).0.to_string(),
            Question::Previous => format!("{:?}", previous(tree, operand)),
            Question::Ends => format!("{:?} {:?}", first(tree, operand), last(tree, operand)),
        })
    })
    .ok_or(())?;
    Ok(Box::leak(answer.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
