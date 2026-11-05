//! The `nodes` service's cases.

use super::service::standard;
use super::types::HtmlNode;
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
    Name(HtmlNode),
    /// How many nodes there are.
    Count,
    /// The standard the names come from.
    Standard,
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("node_is_named_as_the_standard_names_it"),
        word: Word::Succeeds,
        input: || Question::Name(HtmlNode::SourceFile),
        expect: Expect::Value("source-file"),
    },
    Case {
        name: CaseName("the_names_come_from_the_standard"),
        word: Word::Succeeds,
        input: || Question::Standard,
        expect: Expect::Value("WHATWG HTML; parse5 8.0.0; SWC HTML 26.1.0"),
    },
    Case {
        name: CaseName("the_language_has_one_node_so_far"),
        word: Word::Succeeds,
        input: || Question::Count,
        expect: Expect::Value("15"),
    },
];

fn subject(question: Input) -> Value {
    match question {
        Question::Name(each) => kind_name(Kind::Node(each)).0,
        Question::Count => Box::leak(HtmlNode::COUNT.to_string().into_boxed_str()),
        Question::Standard => standard().0,
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
