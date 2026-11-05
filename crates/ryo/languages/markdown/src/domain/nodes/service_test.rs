//! The `nodes` service's cases.

use super::service::standard;
use super::types::MarkdownNode;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::kind_name;
use crate::domain::syntax::types::Kind;
use strum::EnumCount as _;

/// What a row asks: a node's name, how many nodes there are, or the standard.
type Input = Question;
/// The answer, as text.
type Value = &'static str;
type Failure = ();

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The name of a node, as the standard names it.
    Name(MarkdownNode),
    /// How many nodes there are.
    Count,
    /// The standard the names come from.
    Standard,
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_is_named_as_the_type_of_the_standard"),
        word: Word::Succeeds,
        input: || Question::Name(MarkdownNode::ListItem),
        expect: Expect::Value("list-item"),
    },
    Case {
        name: CaseName("the_node_of_the_whole_file_is_root"),
        word: Word::Succeeds,
        input: || Question::Name(MarkdownNode::Root),
        expect: Expect::Value("root"),
    },
    Case {
        name: CaseName("the_names_come_from_the_pinned_mdast"),
        word: Word::Succeeds,
        input: || Question::Standard,
        expect: Expect::Value("mdast c034ec9d"),
    },
    Case {
        name: CaseName("there_are_twenty_five_nodes"),
        word: Word::Succeeds,
        input: || Question::Count,
        expect: Expect::Value("25"),
    },
];

fn subject(question: Input) -> Value {
    match question {
        Question::Name(each) => kind_name(Kind::Node(each)).0,
        Question::Count => Box::leak(MarkdownNode::COUNT.to_string().into_boxed_str()),
        Question::Standard => standard().0,
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
